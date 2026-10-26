use std::collections::{HashMap, HashSet};

use crate::distance::{Distance, Euclidean};
use crate::error::{MarsError, Result};
use crate::graph::{Graph, GraphConfig};
use crate::node::NodeId;
use crate::parser::{BoolConnector, ComparisonOp, ConditionValue, OrderBy, WhereClause};
use crate::schema::{Row, Schema, Value, values_equal, values_compare};

/// A table in the database containing vectors and metadata
pub struct Table {
    pub schema: Schema,
    pub graph: Graph<f32, Euclidean>,
    pub(crate) rows: HashMap<u64, Row>,
    pub(crate) next_id: u64,
    /// Unique constraint indexes: column_name -> set of values
    unique_indexes: HashMap<String, HashSet<String>>,  // Store values as strings for hashing
    row_to_graph: HashMap<u64, NodeId>,
    graph_to_row: HashMap<NodeId, u64>,
}

impl Table {
    pub fn new(schema: Schema, config: GraphConfig) -> Result<Self> {
        let dimension = schema.get_vector_dimension()
            .ok_or_else(|| MarsError::InvalidConfig("Table must have a VECTOR column".into()))?;

        // Initialize unique indexes for columns with UNIQUE constraint
        let unique_indexes: HashMap<String, HashSet<String>> = schema.columns.iter()
            .filter(|c| c.unique)
            .map(|c| (c.name.clone(), HashSet::new()))
            .collect();

        Ok(Table {
            schema,
            graph: Graph::new(dimension, config),
            rows: HashMap::new(),
            next_id: 1,
            unique_indexes,
            row_to_graph: HashMap::new(),
            graph_to_row: HashMap::new(),
        })
    }

    /// Get the table name
    pub fn name(&self) -> &str {
        &self.schema.name
    }

    /// Get the number of rows
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Insert a row with values
    pub fn insert(&mut self, columns: &[String], values: Vec<Value>) -> Result<u64> {
        // Validate and build complete row
        let row_values = self.build_row_values(columns, values)?;
        self.insert_row(row_values)
    }

    /// Insert a row with pre-built values (faster, no validation)
    pub fn insert_row(&mut self, mut row_values: Vec<Value>) -> Result<u64> {
        // Check unique constraints before inserting
        self.check_unique_constraints(&row_values)?;

        // Auto-generate ID
        let id = self.next_id;
        self.next_id += 1;

        // If there's an 'id' column, set it to the auto-generated ID
        if let Some(idx) = self.column_index("id") {
            row_values[idx] = Value::Integer(id as i64);
        }

        // Extract vector
        let vector = self.extract_vector(&row_values)?;

        // Insert into graph
        let graph_id = self.graph.insert(vector);

        // Update unique indexes
        self.update_unique_indexes(&row_values);

        // Create row
        let row = Row::new(id, row_values);
        self.register_graph_mapping(id, graph_id);
        self.rows.insert(id, row);

        Ok(id)
    }

    /// Batch insert multiple rows for better performance.
    /// This is significantly faster than individual inserts.
    pub fn insert_batch(&mut self, rows: Vec<Vec<Value>>) -> Result<Vec<u64>> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Check all unique constraints first
        for row_values in &rows {
            self.check_unique_constraints(row_values)?;
        }

        let batch_size = rows.len();
        let start_id = self.next_id;
        let ids: Vec<u64> = (start_id..start_id + batch_size as u64).collect();
        self.next_id += batch_size as u64;

        // Prepare vectors and rows
        let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(batch_size);
        let mut prepared_rows: Vec<(u64, Vec<Value>)> = Vec::with_capacity(batch_size);

        for (i, mut row_values) in rows.into_iter().enumerate() {
            let id = ids[i];

            // If there's an 'id' column, set it to the auto-generated ID
            if let Some(idx) = self.column_index("id") {
                row_values[idx] = Value::Integer(id as i64);
            }

            // Extract vector
            let vector = self.extract_vector(&row_values)?;
            vectors.push(vector);

            prepared_rows.push((id, row_values));
        }

        // Batch insert into graph
        let graph_ids = self.graph.insert_batch(vectors);

        // Insert all rows and update unique indexes
        for ((id, row_values), graph_id) in prepared_rows.into_iter().zip(graph_ids.into_iter()) {
            self.update_unique_indexes(&row_values);
            let row = Row::new(id, row_values);
            self.register_graph_mapping(id, graph_id);
            self.rows.insert(id, row);
        }

        Ok(ids)
    }

    /// Restore deserialized rows while rebuilding graph and UNIQUE metadata.
    pub fn restore_rows(&mut self, rows: Vec<Row>) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        self.graph = Graph::new(self.graph.dimension(), self.graph.config().clone());
        self.rows.clear();
        self.row_to_graph.clear();
        self.graph_to_row.clear();
        for unique_set in self.unique_indexes.values_mut() {
            unique_set.clear();
        }

        let mut prepared_rows = Vec::with_capacity(rows.len());
        let mut vectors = Vec::with_capacity(rows.len());

        for row in rows {
            self.check_unique_constraints(&row.values)?;
            vectors.push(self.extract_vector(&row.values)?);
            self.update_unique_indexes(&row.values);
            prepared_rows.push(row);
        }

        let graph_ids = self.graph.insert_batch(vectors);
        for (row, graph_id) in prepared_rows.into_iter().zip(graph_ids.into_iter()) {
            self.register_graph_mapping(row.id, graph_id);
            self.rows.insert(row.id, row);
        }

        Ok(())
    }

    /// Select rows matching conditions
    pub fn select(
        &self,
        columns: &[String],
        where_clause: Option<&WhereClause>,
        limit: Option<usize>,
        offset: Option<usize>,
        order_by: Option<&OrderBy>,
        distinct: bool,
    ) -> Vec<Row> {
        let mut results: Vec<&Row> = self.rows.values()
            .filter(|row| self.matches_where(row, where_clause))
            .collect();

        // Apply ORDER BY
        if let Some(ob) = order_by {
            if let Some(idx) = self.column_index(&ob.column) {
                results.sort_by(|a, b| {
                    let cmp = self.values_compare(&a.values[idx], &b.values[idx])
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if ob.ascending { cmp } else { cmp.reverse() }
                });
            }
        }

        // Apply OFFSET
        if let Some(n) = offset {
            results = results.into_iter().skip(n).collect();
        }

        // Apply LIMIT
        if let Some(n) = limit {
            results.truncate(n);
        }

        // Apply DISTINCT
        if distinct {
            let mut seen = std::collections::HashSet::new();
            results = results.into_iter()
                .filter(|row| {
                    seen.insert(row.values_hash())
                })
                .collect();
        }

        // Project columns
        results.into_iter()
            .map(|row| self.project_row(row, columns))
            .collect()
    }

    /// Select by vector similarity
    pub fn select_by_similarity(
        &self,
        query_vector: &[f32],
        k: usize,
        ef_search: usize,
    ) -> Result<Vec<(Row, f32)>> {
        self.select_by_similarity_filtered(query_vector, k, ef_search, None)
    }

    /// Select by vector similarity while applying non-vector filters.
    pub fn select_by_similarity_filtered(
        &self,
        query_vector: &[f32],
        k: usize,
        _ef_search: usize,
        where_clause: Option<&WhereClause>,
    ) -> Result<Vec<(Row, f32)>> {
        self.validate_query_vector(query_vector)?;

        let vector_idx = self.vector_column_index()
            .ok_or_else(|| MarsError::InvalidConfig("Vector column not found".into()))?;

        let mut results: Vec<(Row, f32)> = self.rows.values()
            .filter(|row| self.is_searchable_similarity_row(row, where_clause, vector_idx))
            .filter_map(|row| {
                row.values[vector_idx].as_vector().map(|vector| {
                    (self.project_row(row, &[]), Euclidean::compute(query_vector, vector))
                })
            })
            .collect();

        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        Ok(results)
    }

    /// Update rows matching conditions
    /// NOTE: If the vector column is updated, the graph neighbor edges are NOT updated
    /// to reflect the new vector position. This is a known limitation - vector updates
    /// should be implemented by deleting and re-inserting the row for correct search results.
    pub fn update(
        &mut self,
        assignments: &[(String, Value)],
        where_clause: Option<&WhereClause>,
    ) -> Result<usize> {
        let matching_ids: Vec<u64> = self.rows.values()
            .filter(|row| self.matches_where(row, where_clause))
            .map(|row| row.id)
            .collect();

        // Precompute column indices
        let assignment_indices: Vec<(Option<usize>, Value)> = assignments.iter()
            .map(|(col_name, value)| (self.column_index(col_name), value.clone()))
            .collect();

        let count = matching_ids.len();

        for id in matching_ids {
            if let Some(row) = self.rows.get_mut(&id) {
                for (idx_opt, value) in &assignment_indices {
                    if let Some(idx) = idx_opt {
                        row.values[*idx] = value.clone();
                    }
                }
            }
        }

        Ok(count)
    }

    /// Delete rows matching conditions
    pub fn delete(
        &mut self,
        where_clause: Option<&WhereClause>,
    ) -> Result<usize> {
        let matching_ids: Vec<u64> = self.rows.values()
            .filter(|row| self.matches_where(row, where_clause))
            .map(|row| row.id)
            .collect();

        let count = matching_ids.len();

        for id in &matching_ids {
            // Collect value strings to remove from unique indexes (need to do this before mutable borrow)
            let unique_values_to_remove: Vec<(String, String)> = if !self.unique_indexes.is_empty() {
                if let Some(row) = self.rows.get(id) {
                    row.values.iter().enumerate()
                        .filter_map(|(col_idx, val)| {
                            let col_name = self.schema.columns.get(col_idx)?.name.clone();
                            if self.unique_indexes.contains_key(&col_name) {
                                let value_str = Self::value_to_string(val);
                                if value_str != "NULL" {
                                    return Some((col_name, value_str));
                                }
                            }
                            None
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            self.rows.remove(id);
            if let Some(graph_id) = self.row_to_graph.remove(id) {
                self.graph_to_row.remove(&graph_id);
                self.graph.delete(graph_id);
            }

            // Clean up unique indexes after row removal (borrow is released)
            for (col_name, value_str) in unique_values_to_remove {
                if let Some(unique_set) = self.unique_indexes.get_mut(&col_name) {
                    unique_set.remove(&value_str);
                }
            }
        }

        Ok(count)
    }

    /// Get a row by ID
    pub fn get(&self, id: u64) -> Option<&Row> {
        self.rows.get(&id)
    }

    /// Build row values from column names and provided values
    fn build_row_values(&self, columns: &[String], values: Vec<Value>) -> Result<Vec<Value>> {
        let mut row_values: Vec<Value> = self.schema.columns.iter()
            .map(|_| Value::Null)
            .collect();

        for (i, col_name) in columns.iter().enumerate() {
            let idx = self.column_index(col_name)
                .ok_or_else(|| MarsError::InvalidFormat(format!("Unknown column: {}", col_name)))?;

            row_values[idx] = values.get(i)
                .ok_or_else(|| MarsError::InvalidFormat(format!("Missing value for column: {}", col_name)))?
                .clone();
        }

        Ok(row_values)
    }

    /// Extract vector from row values, validating dimension matches schema
    fn extract_vector(&self, values: &[Value]) -> Result<Vec<f32>> {
        let vec_col = self.schema.vector_column.as_ref()
            .ok_or_else(|| MarsError::InvalidConfig("No vector column defined".into()))?;

        let idx = self.column_index(vec_col)
            .ok_or_else(|| MarsError::InvalidConfig("Vector column not found".into()))?;

        match &values[idx] {
            Value::Vector(v) => {
                let expected_dim = self.graph.dimension();
                if v.len() != expected_dim {
                    return Err(MarsError::InvalidFormat(format!(
                        "Vector dimension mismatch: expected {}, got {}",
                        expected_dim, v.len()
                    )));
                }
                Ok(v.clone())
            }
            _ => Err(MarsError::InvalidFormat("Vector column must contain a vector".into())),
        }
    }

    /// Get column index by name
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.schema.columns.iter().position(|c| c.name == name)
    }

    fn vector_column_index(&self) -> Option<usize> {
        self.schema.vector_column.as_ref()
            .and_then(|name| self.column_index(name))
    }

    fn validate_query_vector(&self, query_vector: &[f32]) -> Result<()> {
        let expected_dim = self.graph.dimension();
        if query_vector.len() != expected_dim {
            return Err(MarsError::DimensionMismatch {
                expected: expected_dim,
                actual: query_vector.len(),
            });
        }

        Ok(())
    }

    fn is_searchable_similarity_row(
        &self,
        row: &Row,
        where_clause: Option<&WhereClause>,
        vector_idx: usize,
    ) -> bool {
        if !self.matches_where(row, where_clause) {
            return false;
        }

        if let Some(parent_idx) = self.column_index("parent_doc_id") {
            if matches!(row.values[parent_idx], Value::Null) {
                return false;
            }
        }

        let Some(vector) = row.values[vector_idx].as_vector() else {
            return false;
        };

        !Self::is_zero_vector(vector)
    }

    fn is_zero_vector(vector: &[f32]) -> bool {
        vector.iter().all(|value| *value == 0.0)
    }

    /// Check if a row matches where clause (supports AND/OR)
    pub fn matches_where(&self, row: &Row, where_clause: Option<&WhereClause>) -> bool {
        match where_clause {
            None => true,
            Some(wc) => {
                if wc.conditions.is_empty() {
                    return true;
                }

                // Evaluate conditions with AND/OR connectors
                let mut result = self.matches_condition(row, &wc.conditions[0]);

                for (i, connector) in wc.connectors.iter().enumerate() {
                    let cond_result = self.matches_condition(row, &wc.conditions[i + 1]);
                    result = match connector {
                        BoolConnector::And => result && cond_result,
                        BoolConnector::Or => result || cond_result,
                    };
                }

                result
            }
        }
    }

    /// Check if a row matches a single condition
    fn matches_condition(&self, row: &Row, cond: &crate::parser::Condition) -> bool {
        let idx = match self.column_index(&cond.column) {
            Some(i) => i,
            None => return false,
        };

        let row_val = &row.values[idx];
        self.evaluate_condition(row_val, &cond.operator, &cond.value)
    }

    /// Evaluate a condition against a value
    fn evaluate_condition(&self, row_val: &Value, op: &ComparisonOp, cond_val: &ConditionValue) -> bool {
        match op {
            ComparisonOp::Eq => {
                if let ConditionValue::Single(v) = cond_val {
                    self.values_equal(row_val, v)
                } else {
                    false
                }
            }
            ComparisonOp::Ne => {
                if let ConditionValue::Single(v) = cond_val {
                    !self.values_equal(row_val, v)
                } else {
                    true
                }
            }
            ComparisonOp::Lt => {
                if let ConditionValue::Single(v) = cond_val {
                    self.values_compare(row_val, v) == Some(std::cmp::Ordering::Less)
                } else {
                    false
                }
            }
            ComparisonOp::Le => {
                if let ConditionValue::Single(v) = cond_val {
                    self.values_compare(row_val, v).map(|o| o != std::cmp::Ordering::Greater).unwrap_or(false)
                } else {
                    false
                }
            }
            ComparisonOp::Gt => {
                if let ConditionValue::Single(v) = cond_val {
                    self.values_compare(row_val, v) == Some(std::cmp::Ordering::Greater)
                } else {
                    false
                }
            }
            ComparisonOp::Ge => {
                if let ConditionValue::Single(v) = cond_val {
                    self.values_compare(row_val, v).map(|o| o != std::cmp::Ordering::Less).unwrap_or(false)
                } else {
                    false
                }
            }
            ComparisonOp::Similar => false, // Handled separately
            ComparisonOp::Like => {
                if let ConditionValue::Single(Value::Text(pattern)) = cond_val {
                    self.match_like(row_val, pattern)
                } else {
                    false
                }
            }
            ComparisonOp::NotLike => {
                if let ConditionValue::Single(Value::Text(pattern)) = cond_val {
                    !self.match_like(row_val, pattern)
                } else {
                    true
                }
            }
            ComparisonOp::In => {
                if let ConditionValue::List(values) = cond_val {
                    values.iter().any(|v| self.values_equal(row_val, v))
                } else {
                    false
                }
            }
            ComparisonOp::NotIn => {
                if let ConditionValue::List(values) = cond_val {
                    !values.iter().any(|v| self.values_equal(row_val, v))
                } else {
                    true
                }
            }
            ComparisonOp::Between => {
                if let ConditionValue::Range(low, high) = cond_val {
                    self.values_compare(row_val, low).map(|o| o != std::cmp::Ordering::Less).unwrap_or(false)
                        && self.values_compare(row_val, high).map(|o| o != std::cmp::Ordering::Greater).unwrap_or(false)
                } else {
                    false
                }
            }
            ComparisonOp::NotBetween => {
                if let ConditionValue::Range(low, high) = cond_val {
                    !(self.values_compare(row_val, low).map(|o| o != std::cmp::Ordering::Less).unwrap_or(false)
                        && self.values_compare(row_val, high).map(|o| o != std::cmp::Ordering::Greater).unwrap_or(false))
                } else {
                    true
                }
            }
            ComparisonOp::IsNull => {
                matches!(row_val, Value::Null)
            }
            ComparisonOp::IsNotNull => {
                !matches!(row_val, Value::Null)
            }
        }
    }

    /// Match LIKE pattern (supports % and _)
    fn match_like(&self, value: &Value, pattern: &str) -> bool {
        match value {
            Value::Text(s) => self.like_match(s, pattern),
            _ => false,
        }
    }

    /// Simple LIKE pattern matching
    fn like_match(&self, text: &str, pattern: &str) -> bool {
        // Convert SQL LIKE pattern to regex-like matching
        let text_lower = text.to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        // Handle common cases efficiently
        if pattern_lower.starts_with('%') && pattern_lower.ends_with('%') {
            // %pattern% - contains
            let middle = &pattern_lower[1..pattern_lower.len()-1];
            text_lower.contains(middle)
        } else if pattern_lower.starts_with('%') {
            // %pattern - ends with
            let suffix = &pattern_lower[1..];
            text_lower.ends_with(suffix)
        } else if pattern_lower.ends_with('%') {
            // pattern% - starts with
            let prefix = &pattern_lower[..pattern_lower.len()-1];
            text_lower.starts_with(prefix)
        } else if pattern_lower.contains('%') || pattern_lower.contains('_') {
            // Complex pattern - use simple wildcard matching
            self.wildcard_match(&text_lower, &pattern_lower)
        } else {
            // Exact match
            text_lower == pattern_lower
        }
    }

    /// Wildcard matching for LIKE patterns
    fn wildcard_match(&self, text: &str, pattern: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        let mut ti = 0;
        let mut pi = 0;
        let mut star_ti = None;
        let mut star_pi = None;

        while ti < text_chars.len() {
            if pi < pattern_chars.len() && (pattern_chars[pi] == '_' || pattern_chars[pi].to_lowercase().next() == text_chars[ti].to_lowercase().next()) {
                ti += 1;
                pi += 1;
            } else if pi < pattern_chars.len() && pattern_chars[pi] == '%' {
                star_ti = Some(ti);
                star_pi = Some(pi);
                pi += 1;
            } else if let Some(star) = star_ti {
                ti = star + 1;
                star_ti = Some(ti);
                pi = star_pi.unwrap() + 1;
            } else {
                return false;
            }
        }

        while pi < pattern_chars.len() && pattern_chars[pi] == '%' {
            pi += 1;
        }

        pi == pattern_chars.len()
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        values_equal(a, b)
    }

    fn register_graph_mapping(&mut self, row_id: u64, graph_id: NodeId) {
        self.row_to_graph.insert(row_id, graph_id);
        self.graph_to_row.insert(graph_id, row_id);
    }

    pub fn values_compare(&self, a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
        values_compare(a, b)
    }

    /// Project row to specified columns
    fn project_row(&self, row: &Row, columns: &[String]) -> Row {
        if columns.is_empty() {
            return row.clone(); // SELECT *
        }

        let values: Vec<Value> = columns.iter()
            .filter_map(|name| {
                self.column_index(name).map(|idx| row.values[idx].clone())
            })
            .collect();

        Row::new(row.id, values)
    }

    // ==================== UNIQUE CONSTRAINT HELPERS ====================

    /// Convert a Value to a string for hashing in unique index
    pub fn value_to_string(value: &Value) -> String {
        match value {
            Value::Null => "NULL".to_string(),
            Value::Integer(i) => format!("I:{}", i),
            Value::Float(f) => format!("F:{}", f),
            Value::Text(s) => format!("T:{}", s),
            Value::Boolean(b) => format!("B:{}", b),
            Value::Vector(v) => format!("V:{}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
            Value::Blob(b) => format!("L:{}", b.len()),
        }
    }

    /// Check if inserting these values would violate any unique constraints
    fn check_unique_constraints(&self, row_values: &[Value]) -> Result<()> {
        for (col_name, unique_set) in &self.unique_indexes {
            if let Some(col_idx) = self.column_index(col_name) {
                let value_str = Self::value_to_string(&row_values[col_idx]);
                // Skip NULL values - they don't count for uniqueness
                if value_str != "NULL" && unique_set.contains(&value_str) {
                    return Err(MarsError::InvalidFormat(format!(
                        "Duplicate value for UNIQUE column '{}'", col_name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Update unique indexes after a successful insert
    fn update_unique_indexes(&mut self, row_values: &[Value]) {
        // First, collect the (column_name, column_index) pairs
        let col_indices: Vec<(String, usize, String)> = self.unique_indexes.keys()
            .filter_map(|col_name| {
                self.column_index(col_name).map(|idx| {
                    (col_name.clone(), idx, Self::value_to_string(&row_values[idx]))
                })
            })
            .collect();

        // Then, update the unique indexes
        for (col_name, _idx, value_str) in col_indices {
            if value_str != "NULL" {
                if let Some(unique_set) = self.unique_indexes.get_mut(&col_name) {
                    unique_set.insert(value_str);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ColumnType;

    fn create_test_schema() -> Schema {
        Schema::new("test")
            .column("id", ColumnType::Integer)
            .column("embedding", ColumnType::Vector(3))
            .column("title", ColumnType::Text)
    }

    #[test]
    fn test_table_creation() {
        let schema = create_test_schema();
        let table = Table::new(schema, GraphConfig::default()).unwrap();

        assert_eq!(table.name(), "test");
        assert!(table.is_empty());
    }

    #[test]
    fn test_insert() {
        let schema = create_test_schema();
        let mut table = Table::new(schema, GraphConfig::default()).unwrap();

        let id = table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![1.0, 2.0, 3.0]), Value::Text("Test".to_string())],
        ).unwrap();

        assert_eq!(id, 1);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_select() {
        let schema = create_test_schema();
        let mut table = Table::new(schema, GraphConfig::default()).unwrap();

        table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![1.0, 0.0, 0.0]), Value::Text("First".to_string())],
        ).unwrap();

        table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![0.0, 1.0, 0.0]), Value::Text("Second".to_string())],
        ).unwrap();

        let rows = table.select(&[], None, None, None, None, false);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_similarity_after_delete_and_reinsert() {
        let schema = create_test_schema();
        let mut table = Table::new(schema, GraphConfig::default()).unwrap();

        let first_id = table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![1.0, 0.0, 0.0]), Value::Text("First".to_string())],
        ).unwrap();
        let second_id = table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![0.0, 1.0, 0.0]), Value::Text("Second".to_string())],
        ).unwrap();

        table.delete(Some(&WhereClause {
            conditions: vec![crate::parser::Condition {
                column: "id".to_string(),
                operator: ComparisonOp::Eq,
                value: ConditionValue::Single(Value::Integer(first_id as i64)),
            }],
            connectors: vec![],
        })).unwrap();

        let third_id = table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![1.0, 0.0, 0.0]), Value::Text("Third".to_string())],
        ).unwrap();

        let results = table.select_by_similarity(&[1.0, 0.0, 0.0], 1, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, third_id);
        assert_ne!(results[0].0.id, second_id);
    }

    #[test]
    fn test_similarity_filtered_excludes_parent_rows_and_zero_vectors() {
        let mut schema = Schema::new("docs");
        schema.columns.push(crate::schema::Column::new("id", ColumnType::Integer).primary_key());
        schema.columns.push(crate::schema::Column::new("embedding", ColumnType::Vector(3)));
        schema.columns.push(crate::schema::Column::new("title", ColumnType::Text));
        schema.columns.push(crate::schema::Column::new("parent_doc_id", ColumnType::Integer));
        schema.vector_column = Some("embedding".to_string());

        let mut table = Table::new(schema, GraphConfig::default()).unwrap();
        table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![0.0, 0.0, 0.0]), Value::Text("Parent".to_string())],
        ).unwrap();
        let child_id = table.insert(
            &["embedding".to_string(), "title".to_string(), "parent_doc_id".to_string()],
            vec![
                Value::Vector(vec![1.0, 0.0, 0.0]),
                Value::Text("Child".to_string()),
                Value::Integer(1),
            ],
        ).unwrap();

        let results = table.select_by_similarity(&[1.0, 0.0, 0.0], 2, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, child_id);
    }

    #[test]
    fn test_similarity_filtered_respects_non_similarity_where_clause() {
        let schema = create_test_schema();
        let mut table = Table::new(schema, GraphConfig::default()).unwrap();
        table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![1.0, 0.0, 0.0]), Value::Text("Allowed".to_string())],
        ).unwrap();
        table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![0.9, 0.0, 0.0]), Value::Text("Blocked".to_string())],
        ).unwrap();

        let where_clause = WhereClause {
            conditions: vec![crate::parser::Condition {
                column: "title".to_string(),
                operator: ComparisonOp::Eq,
                value: ConditionValue::Single(Value::Text("Allowed".to_string())),
            }],
            connectors: vec![],
        };

        let results = table.select_by_similarity_filtered(&[1.0, 0.0, 0.0], 2, 10, Some(&where_clause)).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.values[2], Value::Text("Allowed".to_string()));
    }

    #[test]
    fn test_similarity_dimension_mismatch_returns_error() {
        let schema = create_test_schema();
        let table = Table::new(schema, GraphConfig::default()).unwrap();

        let err = table.select_by_similarity(&[1.0, 0.0], 1, 10).unwrap_err();
        assert!(matches!(err, MarsError::DimensionMismatch { expected: 3, actual: 2 }));
    }

    #[test]
    fn test_restore_rows_rebuilds_unique_indexes() {
        let mut schema = Schema::new("test");
        let mut id_col = crate::schema::Column::new("id", ColumnType::Integer);
        id_col.primary_key = true;
        let embedding_col = crate::schema::Column::new("embedding", ColumnType::Vector(3));
        let mut title_col = crate::schema::Column::new("title", ColumnType::Text);
        title_col.unique = true;
        schema.columns.push(id_col);
        schema.columns.push(embedding_col);
        schema.columns.push(title_col);
        schema.vector_column = Some("embedding".to_string());

        let mut table = Table::new(schema, GraphConfig::default()).unwrap();
        table.restore_rows(vec![Row::new(
            1,
            vec![
                Value::Integer(1),
                Value::Vector(vec![1.0, 0.0, 0.0]),
                Value::Text("duplicate".to_string()),
            ],
        )]).unwrap();

        let err = table.insert(
            &["embedding".to_string(), "title".to_string()],
            vec![Value::Vector(vec![0.0, 1.0, 0.0]), Value::Text("duplicate".to_string())],
        ).unwrap_err();

        assert!(format!("{}", err).contains("UNIQUE"));
    }
}
