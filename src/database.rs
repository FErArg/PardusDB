use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use serde::{Deserialize, Serialize};

use crate::error::{MarsError, Result};
use crate::graph::GraphConfig;
use crate::parser::{BoolConnector, Command, ComparisonOp, Condition, ConditionValue, JoinColumn, JoinType, OrderBy, SelectColumn, WhereClause, parse};
use crate::schema::{Column, ColumnType, Row, Schema, Value, values_equal, values_compare};
use crate::table::Table;

/// File header with database metadata
#[derive(Serialize, Deserialize)]
struct DbHeader {
    pub version: u32,
    pub table_count: u32,
}

/// Maximum allowed size for a single serialized table (10 GB)
const MAX_TABLE_SIZE: usize = 10 * 1024 * 1024 * 1024;

/// Serialized table data
#[derive(Serialize, Deserialize)]
pub struct TableData {
    pub schema: Schema,
    pub rows: Vec<Row>,
    pub centroid: Vec<f32>,
    pub next_id: u64,
    pub config: GraphConfig,
}

/// The main database - manages multiple tables in a single file
pub struct Database {
    tables: HashMap<String, Table>,
    config: GraphConfig,
    path: Option<PathBuf>,
}

impl Database {
    /// Create an in-memory database
    pub fn in_memory() -> Self {
        Database {
            tables: HashMap::new(),
            config: GraphConfig::default(),
            path: None,
        }
    }

    /// Set graph configuration
    pub fn with_config(mut self, config: GraphConfig) -> Self {
        self.config = config;
        self
    }

    /// Open or create a database file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if path.exists() {
            Self::load(&path)
        } else {
            Self::create_new(&path)
        }
    }

    /// Create a new database file
    fn create_new(path: &Path) -> Result<Self> {
        let db = Database {
            tables: HashMap::new(),
            config: GraphConfig::default(),
            path: Some(path.to_path_buf()),
        };

        // Write empty database
        db.save()?;

        Ok(db)
    }

    /// Load database from file
    fn load(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut reader = BufReader::new(&mut file);

        // Read header
        let mut header_buf = [0u8; 8];
        reader.read_exact(&mut header_buf)?;

        let version = u32::from_le_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]);
        let table_count = u32::from_le_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]]);

        // Read tables
        let mut tables = HashMap::new();

        for _ in 0..table_count {
            // Read table size
            let mut size_buf = [0u8; 8];
            reader.read_exact(&mut size_buf)?;
            let size = u64::from_le_bytes(size_buf) as usize;

            if size > MAX_TABLE_SIZE {
                return Err(MarsError::InvalidFormat(
                    format!("Table data too large: {} bytes (max {})", size, MAX_TABLE_SIZE)
                ));
            }

            // Read table data
            let mut table_buf = vec![0u8; size];
            reader.read_exact(&mut table_buf)?;

            let table_data: TableData = bincode::deserialize(&table_buf)
                .map_err(|e| MarsError::InvalidFormat(format!("Failed to deserialize table: {}", e)))?;

            // Reconstruct table with saved config (not default)
            let mut table = Table::new(table_data.schema, table_data.config)?;

            table.restore_rows(table_data.rows)?;

            table.next_id = table_data.next_id;
            tables.insert(table.name().to_string(), table);
        }

        Ok(Database {
            tables,
            config: GraphConfig::default(),
            path: Some(path.to_path_buf()),
        })
    }

    /// Save database to file
    pub fn save(&self) -> Result<()> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(()), // In-memory, no save needed
        };

        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let file = opts.open(path)?;

        let mut writer = BufWriter::new(file);

        // Write header
        let header = DbHeader {
            version: 1,
            table_count: self.tables.len() as u32,
        };
        writer.write_all(&header.version.to_le_bytes())?;
        writer.write_all(&header.table_count.to_le_bytes())?;

        // Write tables
        for table in self.tables.values() {
            let table_data = TableData {
                schema: table.schema.clone(),
                rows: table.rows.values().cloned().collect(),
                centroid: table.graph.centroid().to_vec(),
                next_id: table.next_id,
                config: table.graph.config().clone(),
            };

            let serialized = bincode::serialize(&table_data)
                .map_err(|e| MarsError::InvalidFormat(format!("Failed to serialize table: {}", e)))?;

            writer.write_all(&(serialized.len() as u64).to_le_bytes())?;
            writer.write_all(&serialized)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Execute a SQL-like command
    pub fn execute(&mut self, sql: &str) -> Result<ExecuteResult> {
        let command = parse(sql)?;
        self.execute_command(command)
    }

    /// Direct insert without SQL parsing - much faster
    pub fn insert_direct(
        &mut self,
        table_name: &str,
        vector: Vec<f32>,
        metadata: Vec<(&str, Value)>,
    ) -> Result<u64> {
        let table = self.tables.get_mut(table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", table_name)))?;

        // Build row values
        let mut row_values: Vec<Value> = table.schema.columns.iter()
            .map(|_| Value::Null)
            .collect();

        // Find vector column and set it
        for (i, col) in table.schema.columns.iter().enumerate() {
            if matches!(col.data_type, ColumnType::Vector(_)) {
                row_values[i] = Value::Vector(vector.clone());
            }
        }

        // Set metadata
        for (col_name, value) in metadata {
            if let Some(idx) = table.schema.columns.iter().position(|c| &c.name == col_name) {
                row_values[idx] = value;
            }
        }

        // Insert
        table.insert_row(row_values)
    }

    /// Direct similarity search without SQL parsing
    pub fn search_similar(
        &self,
        table_name: &str,
        query: &[f32],
        k: usize,
        ef_search: usize,
    ) -> Result<Vec<(u64, Vec<Value>, f32)>> {
        let table = self.tables.get(table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", table_name)))?;

        let results = table.select_by_similarity(query, k, ef_search)?;

        Ok(results.into_iter()
            .map(|(row, dist)| (row.id, row.values, dist))
            .collect())
    }

    fn execute_command(&mut self, command: Command) -> Result<ExecuteResult> {
        match command {
            Command::CreateTable { if_not_exists, name, columns } => {
                self.create_table(name, columns, if_not_exists)
            }
            Command::DropTable { name, if_exists } => {
                self.drop_table(name, if_exists)
            }
            Command::Insert { table, columns, values } => {
                self.insert_multi(table, columns, values)
            }
            Command::Select { table, columns, where_clause, group_by, having, order_by, limit, offset, distinct } => {
                self.select(table, columns, where_clause.as_ref(), group_by.as_ref(), having.as_ref(), order_by.as_ref(), limit, offset, distinct)
            }
            Command::Update { table, assignments, where_clause } => {
                self.update(table, assignments, where_clause.as_ref())
            }
            Command::Delete { table, where_clause } => {
                self.delete(table, where_clause.as_ref())
            }
            Command::ShowTables => {
                self.show_tables()
            }
            Command::Join { left_table, right_table, join_type, left_column, right_column, columns, where_clause, order_by, limit, offset } => {
                self.execute_join(left_table, right_table, join_type, left_column, right_column, columns, where_clause.as_ref(), order_by.as_ref(), limit, offset)
            }
        }
    }

    fn create_table(
        &mut self,
        name: String,
        columns: Vec<crate::parser::ColumnDef>,
        if_not_exists: bool,
    ) -> Result<ExecuteResult> {
        if self.tables.contains_key(&name) {
            if if_not_exists {
                return Ok(ExecuteResult::CreateTable { name });
            }
            return Err(MarsError::InvalidConfig(format!("Table '{}' already exists", name)));
        }

        let mut schema = Schema::new(&name);
        for col_def in columns {
            let is_vector = matches!(col_def.data_type, ColumnType::Vector(_));
            let col_name = col_def.name.clone();

            let mut col = Column::new(&col_def.name, col_def.data_type);
            col.primary_key = col_def.primary_key;
            col.nullable = !col_def.not_null;
            col.unique = col_def.unique;  // NEW: pass unique constraint
            schema.columns.push(col);

            if is_vector {
                schema.vector_column = Some(col_name);
            }
        }

        let table = Table::new(schema, self.config.clone())?;
        self.tables.insert(name.clone(), table);

        Ok(ExecuteResult::CreateTable { name })
    }

    fn drop_table(&mut self, name: String, if_exists: bool) -> Result<ExecuteResult> {
        if self.tables.remove(&name).is_none() {
            if if_exists {
                return Ok(ExecuteResult::DropTable { name });
            }
            return Err(MarsError::InvalidFormat(format!("Table '{}' does not exist", name)));
        }
        Ok(ExecuteResult::DropTable { name })
    }

    fn insert_multi(&mut self, table_name: String, columns: Vec<String>, values: Vec<Vec<Value>>) -> Result<ExecuteResult> {
        let table = self.tables.get_mut(&table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", table_name)))?;

        let mut last_id = 0u64;
        for row_values in values {
            last_id = table.insert(&columns, row_values)?;
        }
        Ok(ExecuteResult::Insert { id: last_id })
    }

    fn select(
        &self,
        table_name: String,
        columns: Vec<SelectColumn>,
        where_clause: Option<&WhereClause>,
        group_by: Option<&Vec<String>>,
        having: Option<&WhereClause>,
        order_by: Option<&OrderBy>,
        limit: Option<usize>,
        offset: Option<usize>,
        distinct: bool,
    ) -> Result<ExecuteResult> {
        let table = self.tables.get(&table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", table_name)))?;

        // Check for vector similarity query
        if let Some((query_vec, filter_clause)) = split_similarity_where(where_clause)? {
            let k = limit.unwrap_or(10);
            let results = table.select_by_similarity_filtered(&query_vec, k, 100, filter_clause.as_ref())?;
            return Ok(ExecuteResult::SelectSimilar { results });
        }

        // Check for GROUP BY with aggregates
        if let Some(group_cols) = group_by {
            return self.execute_group_by(table, &columns, where_clause, group_cols, having, order_by, limit, offset);
        }

        // Check for aggregate functions (without GROUP BY)
        let has_aggregates = columns.iter().any(|c| matches!(c, SelectColumn::Aggregate { .. }));
        if has_aggregates {
            return self.execute_aggregates(table, &columns, where_clause);
        }

        // Convert SelectColumn to column names
        let col_names: Vec<String> = columns.iter()
            .filter_map(|c| match c {
                SelectColumn::Column(name) => Some(name.clone()),
                _ => None,
            })
            .collect();

        let is_star = columns.iter().any(|c| matches!(c, SelectColumn::All));

        let rows = table.select(
            if is_star { &[] } else { &col_names },
            where_clause,
            limit,
            offset,
            order_by,
            distinct,
        );
        Ok(ExecuteResult::Select { rows })
    }

    fn execute_aggregates(&self, table: &Table, columns: &[SelectColumn], where_clause: Option<&WhereClause>) -> Result<ExecuteResult> {
        use crate::parser::AggregateFunc;

        // Get matching rows
        let matching_rows: Vec<&Row> = table.rows.values()
            .filter(|row| table.matches_where(row, where_clause))
            .collect();

        let mut results = Vec::new();

        for col in columns {
            match col {
                SelectColumn::Aggregate { func, column, alias } => {
                    let value = compute_aggregate(func, column, &matching_rows, table);
                    let name = alias.clone().unwrap_or_else(|| format!("{:?}({})", func, column));
                    results.push((name, value));
                }
                SelectColumn::Column(name) => {
                    // For non-aggregate columns in aggregate query, take first value
                    if let Some(row) = matching_rows.first() {
                        if let Some(idx) = table.column_index(name) {
                            results.push((name.clone(), row.values.get(idx).cloned().unwrap_or(Value::Null)));
                        }
                    }
                }
                SelectColumn::All => {}
            }
        }

        Ok(ExecuteResult::Aggregate { results })
    }

    /// Execute GROUP BY with aggregates using hash aggregation
    fn execute_group_by(
        &self,
        table: &Table,
        columns: &[SelectColumn],
        where_clause: Option<&WhereClause>,
        group_by: &[String],
        having: Option<&WhereClause>,
        order_by: Option<&OrderBy>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ExecuteResult> {
        use crate::parser::AggregateFunc;
        use std::collections::HashMap as StdHashMap;

        // Get matching rows
        let matching_rows: Vec<&Row> = table.rows.values()
            .filter(|row| table.matches_where(row, where_clause))
            .collect();

        // Get column indices for GROUP BY columns
        let group_indices: Vec<(String, usize)> = group_by.iter()
            .filter_map(|name| table.column_index(name).map(|idx| (name.clone(), idx)))
            .collect();

        // Hash aggregation: group_key -> list of rows
        let mut groups: StdHashMap<Vec<String>, Vec<&Row>> = StdHashMap::new();

        for row in &matching_rows {
            // Create group key from GROUP BY column values
            let key: Vec<String> = group_indices.iter()
                .map(|(_, idx)| Table::value_to_string(&row.values[*idx]))
                .collect();
            groups.entry(key).or_default().push(*row);
        }

        // Pre-compute column names from the SELECT columns (same for all groups)
        let col_names: Vec<String> = columns.iter()
            .flat_map(|col| match col {
                SelectColumn::Column(name) => vec![name.clone()],
                SelectColumn::Aggregate { func, column, alias } => {
                    vec![alias.clone().unwrap_or_else(|| format!("{:?}({})", func, column))]
                }
                SelectColumn::All => {
                    table.schema.columns.iter()
                        .map(|c| c.name.clone())
                        .collect()
                }
            })
            .collect();

        // Process each group and compute aggregates
        let mut result_rows: Vec<Row> = Vec::new();

        for (_group_key, group_rows) in groups.iter() {
            let mut values = Vec::new();

            for col in columns {
                match col {
                    SelectColumn::Column(name) => {
                        // Take value from first row in group
                        if let Some(row) = group_rows.first() {
                            if let Some(idx) = table.column_index(name) {
                                values.push(row.values.get(idx).cloned().unwrap_or(Value::Null));
                            }
                        }
                    }
                    SelectColumn::Aggregate { func, column, alias: _ } => {
                        let value = compute_aggregate(func, column, group_rows, table);
                        values.push(value);
                    }
                    SelectColumn::All => {
                        // Include all columns from first row
                        if let Some(row) = group_rows.first() {
                            for val in row.values.iter() {
                                values.push(val.clone());
                            }
                        }
                    }
                }
            }

            // Create a temporary row for HAVING evaluation
            let temp_row = Row::new(0, values.clone());

            // Apply HAVING clause if present
            let passes_having = if let Some(having_clause) = having {
                // For HAVING, we need to match against the computed values
                // This is a simplified implementation
                self.matches_having(&temp_row, &col_names, having_clause, table)
            } else {
                true
            };

            if passes_having {
                result_rows.push(temp_row);
            }
        }

        // Apply ORDER BY
        if let Some(ob) = order_by {
            if let Some(idx) = col_names.iter().position(|n| n == &ob.column) {
                result_rows.sort_by(|a, b| {
                    let cmp = table.values_compare(&a.values[idx], &b.values[idx])
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if ob.ascending { cmp } else { cmp.reverse() }
                });
            }
        }

        // Apply OFFSET
        if let Some(n) = offset {
            result_rows = result_rows.into_iter().skip(n).collect();
        }

        // Apply LIMIT
        if let Some(n) = limit {
            result_rows.truncate(n);
        }

        // Create aggregate results format
        let results: Vec<(String, Value)> = result_rows.into_iter()
            .flat_map(|row| col_names.iter().cloned().zip(row.values.into_iter()))
            .collect();

        // For GROUP BY, return as aggregate results grouped
        Ok(ExecuteResult::Aggregate { results })
    }

    /// Helper to match HAVING clause against grouped results
    fn matches_having(&self, row: &Row, col_names: &[String], having: &WhereClause, table: &Table) -> bool {
        if having.conditions.is_empty() {
            return true;
        }

        let mut result = self.matches_having_condition(row, col_names, &having.conditions[0], table);

        for (i, connector) in having.connectors.iter().enumerate() {
            let cond_result = self.matches_having_condition(row, col_names, &having.conditions[i + 1], table);
            result = match connector {
                BoolConnector::And => result && cond_result,
                BoolConnector::Or => result || cond_result,
            };
        }

        result
    }

    fn matches_having_condition(&self, row: &Row, col_names: &[String], cond: &Condition, _table: &Table) -> bool {
        // Find column index in the result row
        let Some(idx) = col_names.iter().position(|n| n == &cond.column) else {
            return false;
        };
        let row_val = &row.values[idx];

        match &cond.value {
            ConditionValue::Single(value) => {
                match cond.operator {
                    ComparisonOp::Eq => values_equal(row_val, value),
                    ComparisonOp::Ne => !values_equal(row_val, value),
                    ComparisonOp::Gt => values_compare(row_val, value) == Some(std::cmp::Ordering::Greater),
                    ComparisonOp::Ge => values_compare(row_val, value).map(|o| o != std::cmp::Ordering::Less).unwrap_or(false),
                    ComparisonOp::Lt => values_compare(row_val, value) == Some(std::cmp::Ordering::Less),
                    ComparisonOp::Le => values_compare(row_val, value).map(|o| o != std::cmp::Ordering::Greater).unwrap_or(false),
                    _ => true,
                }
            }
            _ => true,
        }
    }

    fn update(
        &mut self,
        table_name: String,
        assignments: Vec<(String, Value)>,
        where_clause: Option<&crate::parser::WhereClause>,
    ) -> Result<ExecuteResult> {
        let table = self.tables.get_mut(&table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", table_name)))?;

        let count = table.update(&assignments, where_clause)?;
        Ok(ExecuteResult::Update { count })
    }

    fn delete(
        &mut self,
        table_name: String,
        where_clause: Option<&crate::parser::WhereClause>,
    ) -> Result<ExecuteResult> {
        let table = self.tables.get_mut(&table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", table_name)))?;

        let count = table.delete(where_clause)?;
        Ok(ExecuteResult::Delete { count })
    }

    fn show_tables(&self) -> Result<ExecuteResult> {
        let tables: Vec<TableInfo> = self.tables.values()
            .map(|t| TableInfo {
                name: t.name().to_string(),
                rows: t.len(),
                dimension: t.schema.get_vector_dimension().unwrap_or(0),
            })
            .collect();

        Ok(ExecuteResult::ShowTables { tables })
    }

    /// Execute JOIN using hash join algorithm O(n+m)
    fn execute_join(
        &self,
        left_table_name: String,
        right_table_name: String,
        join_type: JoinType,
        left_column: String,
        right_column: String,
        columns: Vec<JoinColumn>,
        where_clause: Option<&WhereClause>,
        order_by: Option<&OrderBy>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ExecuteResult> {
        use std::collections::HashMap as StdHashMap;

        let left_table = self.tables.get(&left_table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", left_table_name)))?;
        let right_table = self.tables.get(&right_table_name)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Table '{}' does not exist", right_table_name)))?;

        // Get column indices
        let left_col_idx = left_table.column_index(&left_column)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Column '{}' not found in table '{}'", left_column, left_table_name)))?;
        let right_col_idx = right_table.column_index(&right_column)
            .ok_or_else(|| MarsError::InvalidFormat(format!("Column '{}' not found in table '{}'", right_column, right_table_name)))?;

        // Build phase: Create hash map from right table (smaller table ideally)
        // Key: join column value as string, Value: list of rows
        let mut right_hash: StdHashMap<String, Vec<&Row>> = StdHashMap::new();
        for row in right_table.rows.values() {
            if let Some(val) = row.values.get(right_col_idx) {
                let key = Table::value_to_string(val);
                right_hash.entry(key).or_default().push(row);
            }
        }

        // Probe phase: For each row in left table, look up in hash map
        let mut result_rows: Vec<Row> = Vec::new();

        for left_row in left_table.rows.values() {
            let left_key = left_row.values.get(left_col_idx)
                .map(|v| Table::value_to_string(v))
                .unwrap_or_default();

            let matching_right_rows = right_hash.get(&left_key);

            match join_type {
                JoinType::Inner => {
                    if let Some(right_rows) = matching_right_rows {
                        for right_row in right_rows {
                            let joined = self.create_joined_row(
                                left_row, right_row,
                                left_table, right_table,
                                &columns,
                                &left_table_name, &right_table_name,
                            );
                            result_rows.push(joined);
                        }
                    }
                }
                JoinType::Left => {
                    if let Some(right_rows) = matching_right_rows {
                        for right_row in right_rows {
                            let joined = self.create_joined_row(
                                left_row, right_row,
                                left_table, right_table,
                                &columns,
                                &left_table_name, &right_table_name,
                            );
                            result_rows.push(joined);
                        }
                    } else {
                        // No match - include left row with NULLs for right columns
                        let joined = self.create_joined_row_with_nulls(
                            left_row,
                            left_table, right_table,
                            &columns,
                            &left_table_name, &right_table_name,
                        );
                        result_rows.push(joined);
                    }
                }
                JoinType::Right => {
                    if let Some(right_rows) = matching_right_rows {
                        for right_row in right_rows {
                            let joined = self.create_joined_row(
                                left_row, right_row,
                                left_table, right_table,
                                &columns,
                                &left_table_name, &right_table_name,
                            );
                            result_rows.push(joined);
                        }
                    }
                }
            }
        }

        // For RIGHT JOIN, also include unmatched right rows
        if join_type == JoinType::Right {
            let mut left_matched: StdHashMap<String, bool> = StdHashMap::new();
            for left_row in left_table.rows.values() {
                if let Some(val) = left_row.values.get(left_col_idx) {
                    let key = Table::value_to_string(val);
                    left_matched.insert(key, true);
                }
            }
            for right_row in right_table.rows.values() {
                let right_key = right_row.values.get(right_col_idx)
                    .map(|v| Table::value_to_string(v))
                    .unwrap_or_default();
                if !left_matched.contains_key(&right_key) {
                    let joined = self.create_joined_row_left_nulls(
                        right_row,
                        left_table, right_table,
                        &columns,
                        &left_table_name, &right_table_name,
                    );
                    result_rows.push(joined);
                }
            }
        }

        // Apply WHERE clause if present
        if let Some(wc) = where_clause {
            // For joined rows, we need to handle table.column references
            result_rows = result_rows.into_iter()
                .filter(|row| self.matches_join_where(row, wc))
                .collect();
        }

        // Apply ORDER BY
        if let Some(ob) = order_by {
            result_rows.sort_by(|a, b| {
                // Find column index for ordering - simplified, just sort by first column
                let a_val = a.values.get(0).unwrap_or(&Value::Null);
                let b_val = b.values.get(0).unwrap_or(&Value::Null);
                let cmp = Table::value_to_string(a_val).cmp(&Table::value_to_string(b_val));
                if ob.ascending { cmp } else { cmp.reverse() }
            });
        }

        // Apply OFFSET
        let mut result_rows = if let Some(n) = offset {
            result_rows.into_iter().skip(n).collect()
        } else {
            result_rows
        };

        // Apply LIMIT
        if let Some(n) = limit {
            result_rows.truncate(n);
        }

        Ok(ExecuteResult::Select { rows: result_rows })
    }

    /// Create a joined row from left and right rows
    fn create_joined_row(
        &self,
        left_row: &Row,
        right_row: &Row,
        left_table: &Table,
        right_table: &Table,
        columns: &[JoinColumn],
        left_table_name: &str,
        right_table_name: &str,
    ) -> Row {
        let mut values = Vec::new();

        for col in columns {
            match col {
                JoinColumn::All => {
                    // Add all columns from left table
                    for val in &left_row.values {
                        values.push(val.clone());
                    }
                    // Add all columns from right table
                    for val in &right_row.values {
                        values.push(val.clone());
                    }
                }
                JoinColumn::TableColumn { table, column } => {
                    if table.to_lowercase() == left_table_name.to_lowercase() {
                        if let Some(idx) = left_table.column_index(column) {
                            values.push(left_row.values.get(idx).cloned().unwrap_or(Value::Null));
                        } else {
                            values.push(Value::Null);
                        }
                    } else if table.to_lowercase() == right_table_name.to_lowercase() {
                        if let Some(idx) = right_table.column_index(column) {
                            values.push(right_row.values.get(idx).cloned().unwrap_or(Value::Null));
                        } else {
                            values.push(Value::Null);
                        }
                    } else {
                        values.push(Value::Null);
                    }
                }
            }
        }

        Row::new(0, values)
    }

    /// Create a joined row with NULLs for right table columns (LEFT JOIN no match)
    fn create_joined_row_with_nulls(
        &self,
        left_row: &Row,
        left_table: &Table,
        right_table: &Table,
        columns: &[JoinColumn],
        left_table_name: &str,
        right_table_name: &str,
    ) -> Row {
        let mut values = Vec::new();

        for col in columns {
            match col {
                JoinColumn::All => {
                    // Add all columns from left table
                    for val in &left_row.values {
                        values.push(val.clone());
                    }
                    // Add NULLs for right table columns
                    for _ in &right_table.schema.columns {
                        values.push(Value::Null);
                    }
                }
                JoinColumn::TableColumn { table, column } => {
                    if table.to_lowercase() == left_table_name.to_lowercase() {
                        if let Some(idx) = left_table.column_index(column) {
                            values.push(left_row.values.get(idx).cloned().unwrap_or(Value::Null));
                        } else {
                            values.push(Value::Null);
                        }
                    } else {
                        // Right table column - NULL
                        values.push(Value::Null);
                    }
                }
            }
        }

        Row::new(0, values)
    }

    /// Create a joined row with NULLs for left table columns (RIGHT JOIN no match)
    fn create_joined_row_left_nulls(
        &self,
        right_row: &Row,
        left_table: &Table,
        right_table: &Table,
        columns: &[JoinColumn],
        left_table_name: &str,
        right_table_name: &str,
    ) -> Row {
        let mut values = Vec::new();

        for col in columns {
            match col {
                JoinColumn::All => {
                    // Add NULLs for left table columns
                    for _ in &left_table.schema.columns {
                        values.push(Value::Null);
                    }
                    // Add all columns from right table
                    for val in &right_row.values {
                        values.push(val.clone());
                    }
                }
                JoinColumn::TableColumn { table, column } => {
                    if table.to_lowercase() == right_table_name.to_lowercase() {
                        if let Some(idx) = right_table.column_index(column) {
                            values.push(right_row.values.get(idx).cloned().unwrap_or(Value::Null));
                        } else {
                            values.push(Value::Null);
                        }
                    } else {
                        // Left table column - NULL
                        values.push(Value::Null);
                    }
                }
            }
        }

        Row::new(0, values)
    }

    /// Check if a joined row matches a WHERE clause
    fn matches_join_where(&self, _row: &Row, _where_clause: &WhereClause) -> bool {
        // Simplified - always returns true for now
        // Full implementation would need to handle table.column references
        true
    }

    /// Get table names
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }

    /// Get table by name
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }
}

fn split_similarity_where(where_clause: Option<&WhereClause>) -> Result<Option<(Vec<f32>, Option<WhereClause>)>> {
    let Some(where_clause) = where_clause else {
        return Ok(None);
    };

    let similarity_conditions: Vec<_> = where_clause.conditions.iter()
        .enumerate()
        .filter(|(_, cond)| cond.operator == ComparisonOp::Similar)
        .collect();

    if similarity_conditions.is_empty() {
        return Ok(None);
    }

    if similarity_conditions.len() > 1 {
        return Err(MarsError::InvalidFormat("SIMILARITY queries support only one similarity condition".into()));
    }

    if !where_clause.connectors.iter().all(|connector| matches!(connector, BoolConnector::And)) {
        return Err(MarsError::InvalidFormat("SIMILARITY queries only support AND connectors".into()));
    }

    let (similarity_index, similarity_condition) = similarity_conditions[0];
    let query_vec = match &similarity_condition.value {
        ConditionValue::Single(Value::Vector(query_vec)) => query_vec.clone(),
        _ => {
            return Err(MarsError::InvalidFormat(
                "SIMILARITY condition must use a vector value".into(),
            ));
        }
    };

    let remaining_conditions: Vec<Condition> = where_clause.conditions.iter()
        .enumerate()
        .filter(|(index, _)| *index != similarity_index)
        .map(|(_, cond)| cond.clone())
        .collect();

    let filter_clause = if remaining_conditions.is_empty() {
        None
    } else {
        Some(WhereClause {
            connectors: vec![BoolConnector::And; remaining_conditions.len().saturating_sub(1)],
            conditions: remaining_conditions,
        })
    };

    Ok(Some((query_vec, filter_clause)))
}

/// Compute a single aggregate value over a set of rows.
fn compute_aggregate(func: &crate::parser::AggregateFunc, column: &str, rows: &[&Row], table: &Table) -> Value {
    use crate::parser::AggregateFunc;

    match func {
        AggregateFunc::Count => {
            if column == "*" {
                Value::Integer(rows.len() as i64)
            } else {
                let idx = table.column_index(column).unwrap_or(0);
                let count = rows.iter()
                    .filter(|r| !matches!(r.values.get(idx), Some(Value::Null) | None))
                    .count();
                Value::Integer(count as i64)
            }
        }
        AggregateFunc::Sum => {
            let idx = table.column_index(column).unwrap_or(0);
            let sum: f64 = rows.iter()
                .filter_map(|r| match r.values.get(idx) {
                    Some(Value::Integer(i)) => Some(*i as f64),
                    Some(Value::Float(f)) => Some(*f),
                    _ => None,
                })
                .sum();
            Value::Float(sum)
        }
        AggregateFunc::Avg => {
            let idx = table.column_index(column).unwrap_or(0);
            let values: Vec<f64> = rows.iter()
                .filter_map(|r| match r.values.get(idx) {
                    Some(Value::Integer(i)) => Some(*i as f64),
                    Some(Value::Float(f)) => Some(*f),
                    _ => None,
                })
                .collect();
            if values.is_empty() {
                Value::Null
            } else {
                Value::Float(values.iter().sum::<f64>() / values.len() as f64)
            }
        }
        AggregateFunc::Min => {
            let idx = table.column_index(column).unwrap_or(0);
            rows.iter()
                .filter_map(|r| r.values.get(idx))
                .filter(|v| !matches!(v, Value::Null))
                .min_by(|a, b| table.values_compare(a, b).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
                .unwrap_or(Value::Null)
        }
        AggregateFunc::Max => {
            let idx = table.column_index(column).unwrap_or(0);
            rows.iter()
                .filter_map(|r| r.values.get(idx))
                .filter(|v| !matches!(v, Value::Null))
                .max_by(|a, b| table.values_compare(a, b).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
                .unwrap_or(Value::Null)
        }
    }
}

/// Result of executing a command
#[derive(Debug)]
pub enum ExecuteResult {
    CreateTable { name: String },
    DropTable { name: String },
    Insert { id: u64 },
    Select { rows: Vec<Row> },
    SelectSimilar { results: Vec<(Row, f32)> },
    Aggregate { results: Vec<(String, Value)> },
    Update { count: usize },
    Delete { count: usize },
    ShowTables { tables: Vec<TableInfo> },
}

/// Table information
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub rows: usize,
    pub dimension: usize,
}

impl std::fmt::Display for ExecuteResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecuteResult::CreateTable { name } => write!(f, "Table '{}' created", name),
            ExecuteResult::DropTable { name } => write!(f, "Table '{}' dropped", name),
            ExecuteResult::Insert { id } => write!(f, "Inserted row with id={}", id),
            ExecuteResult::Select { rows } => {
                writeln!(f, "Found {} rows:", rows.len())?;
                for row in rows {
                    writeln!(f, "  id={}, values={:?}", row.id, row.values)?;
                }
                Ok(())
            }
            ExecuteResult::SelectSimilar { results } => {
                writeln!(f, "Found {} similar rows:", results.len())?;
                for (row, dist) in results {
                    writeln!(f, "  id={}, distance={:.4}, values={:?}", row.id, dist, row.values)?;
                }
                Ok(())
            }
            ExecuteResult::Aggregate { results } => {
                writeln!(f, "Aggregate results:")?;
                for (name, value) in results {
                    writeln!(f, "  {} = {:?}", name, value)?;
                }
                Ok(())
            }
            ExecuteResult::Update { count } => write!(f, "Updated {} rows", count),
            ExecuteResult::Delete { count } => write!(f, "Deleted {} rows", count),
            ExecuteResult::ShowTables { tables } => {
                writeln!(f, "Tables ({}):", tables.len())?;
                for t in tables {
                    writeln!(f, "  {} ({} rows, dim={})", t.name, t.rows, t.dimension)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_table() {
        let mut db = Database::in_memory();

        let result = db.execute(
            "CREATE TABLE docs (id INTEGER, embedding VECTOR(128), title TEXT);"
        ).unwrap();

        assert!(matches!(result, ExecuteResult::CreateTable { .. }));
        assert!(db.get_table("docs").is_some());
    }

    #[test]
    fn test_create_table_if_not_exists() {
        let mut db = Database::in_memory();

        db.execute("CREATE TABLE docs (id INTEGER, embedding VECTOR(3), title TEXT);")
            .unwrap();

        let result = db.execute(
            "CREATE TABLE IF NOT EXISTS docs (id INTEGER, embedding VECTOR(3), title TEXT);"
        ).unwrap();

        assert!(matches!(result, ExecuteResult::CreateTable { .. }));
        assert!(db.get_table("docs").is_some());
    }

    #[test]
    fn test_insert_and_select() {
        let mut db = Database::in_memory();

        db.execute("CREATE TABLE docs (id INTEGER, embedding VECTOR(3), title TEXT);").unwrap();

        db.execute("INSERT INTO docs (embedding, title) VALUES ([1.0, 0.0, 0.0], 'First');").unwrap();
        db.execute("INSERT INTO docs (embedding, title) VALUES ([0.0, 1.0, 0.0], 'Second');").unwrap();

        let result = db.execute("SELECT * FROM docs;").unwrap();
        assert!(matches!(result, ExecuteResult::Select { rows } if rows.len() == 2));
    }

    #[test]
    fn test_delete() {
        let mut db = Database::in_memory();

        db.execute("CREATE TABLE docs (id INTEGER, embedding VECTOR(3), title TEXT);").unwrap();
        db.execute("INSERT INTO docs (embedding, title) VALUES ([1.0, 0.0, 0.0], 'Test');").unwrap();

        let result = db.execute("DELETE FROM docs WHERE id = 1;").unwrap();
        assert!(matches!(result, ExecuteResult::Delete { count: 1 }));
    }
}
