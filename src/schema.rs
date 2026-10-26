use serde::{Deserialize, Serialize};

/// Column types for schema definition
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ColumnType {
    Vector(usize),  // VECTOR(dimensions)
    Text,
    Integer,
    Float,
    Boolean,
    Blob,
}

impl ColumnType {
    pub fn is_vector(&self) -> bool {
        matches!(self, ColumnType::Vector(_))
    }
}

impl ColumnType {
    pub fn to_sql(&self) -> String {
        match self {
            ColumnType::Vector(dim) => format!("VECTOR({})", dim),
            ColumnType::Text => "TEXT".to_string(),
            ColumnType::Integer => "INTEGER".to_string(),
            ColumnType::Float => "FLOAT".to_string(),
            ColumnType::Boolean => "BOOLEAN".to_string(),
            ColumnType::Blob => "BLOB".to_string(),
        }
    }
}

/// A column definition in a table
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: ColumnType,
    pub primary_key: bool,
    pub nullable: bool,
    pub unique: bool,  // NEW: UNIQUE constraint
}

impl Column {
    pub fn new(name: &str, data_type: ColumnType) -> Self {
        Column {
            name: name.to_string(),
            data_type,
            primary_key: false,
            nullable: true,
            unique: false,
        }
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }

    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
}

/// A table schema definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub columns: Vec<Column>,
    pub vector_column: Option<String>,
}

impl Schema {
    pub fn new(name: &str) -> Self {
        Schema {
            name: name.to_string(),
            columns: Vec::new(),
            vector_column: None,
        }
    }

    pub fn column(mut self, name: &str, data_type: ColumnType) -> Self {
        if matches!(data_type, ColumnType::Vector(_)) {
            self.vector_column = Some(name.to_string());
        }
        self.columns.push(Column::new(name, data_type));
        self
    }

    pub fn get_vector_column(&self) -> Option<&Column> {
        self.vector_column.as_ref().and_then(|name| {
            self.columns.iter().find(|c| &c.name == name)
        })
    }

    pub fn get_vector_dimension(&self) -> Option<usize> {
        self.get_vector_column().and_then(|c| {
            if let ColumnType::Vector(dim) = c.data_type {
                Some(dim)
            } else {
                None
            }
        })
    }

    pub fn to_sql(&self) -> String {
        let cols: Vec<String> = self.columns.iter().map(|c| {
            let mut s = format!("{} {}", c.name, c.data_type.to_sql());
            if c.primary_key {
                s.push_str(" PRIMARY KEY");
            }
            if c.unique {
                s.push_str(" UNIQUE");
            }
            if !c.nullable && !c.primary_key {
                s.push_str(" NOT NULL");
            }
            s
        }).collect();

        format!("CREATE TABLE {} (\n    {}\n);", self.name, cols.join(",\n    "))
    }
}

/// A row value - can hold different types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Vector(Vec<f32>),
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Blob(Vec<u8>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_vector(&self) -> Option<&[f32]> {
        match self {
            Value::Vector(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Compute a stable hash for this value (used for DISTINCT and unique indexes).
    pub fn stable_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Value::Null => {
                0u64.hash(&mut hasher);
            }
            Value::Vector(v) => {
                for (i, &f) in v.iter().enumerate() {
                    // Mix index with value for stable hash
                    (i, f.to_bits()).hash(&mut hasher);
                }
            }
            Value::Text(s) => {
                s.hash(&mut hasher);
            }
            Value::Integer(i) => {
                i.hash(&mut hasher);
            }
            Value::Float(f) => {
                f.to_bits().hash(&mut hasher);
            }
            Value::Boolean(b) => {
                b.hash(&mut hasher);
            }
            Value::Blob(b) => {
                b.hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

/// A row in a table
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row {
    pub id: u64,
    pub values: Vec<Value>,
}

impl Row {
    pub fn new(id: u64, values: Vec<Value>) -> Self {
        Row { id, values }
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Compute a stable hash of all values in this row (used for DISTINCT).
    pub fn values_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for val in &self.values {
            hasher.write_u64(val.stable_hash());
        }
        hasher.finish()
    }
}

/// Compare two values for equality (handles cross-type comparisons).
pub fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Integer(i1), Value::Integer(i2)) => i1 == i2,
        (Value::Float(f1), Value::Float(f2)) => f1 == f2,
        (Value::Text(s1), Value::Text(s2)) => s1 == s2,
        (Value::Boolean(b1), Value::Boolean(b2)) => b1 == b2,
        (Value::Integer(i), Value::Float(f)) => (*i as f64) == *f,
        (Value::Float(f), Value::Integer(i)) => *f == (*i as f64),
        _ => false,
    }
}

/// Compare two values for ordering (handles cross-type comparisons).
pub fn values_compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Integer(i1), Value::Integer(i2)) => i1.partial_cmp(i2),
        (Value::Float(f1), Value::Float(f2)) => f1.partial_cmp(f2),
        (Value::Text(s1), Value::Text(s2)) => s1.partial_cmp(s2),
        (Value::Integer(i), Value::Float(f)) => (*i as f64).partial_cmp(f),
        (Value::Float(f), Value::Integer(i)) => f.partial_cmp(&(*i as f64)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = Schema::new("documents")
            .column("id", ColumnType::Integer)
            .column("embedding", ColumnType::Vector(768))
            .column("title", ColumnType::Text)
            .column("score", ColumnType::Float);

        assert_eq!(schema.columns.len(), 4);
        assert_eq!(schema.get_vector_dimension(), Some(768));
    }

    #[test]
    fn test_schema_to_sql() {
        let schema = Schema::new("documents")
            .column("id", ColumnType::Integer)
            .column("embedding", ColumnType::Vector(768))
            .column("title", ColumnType::Text);

        let sql = schema.to_sql();
        assert!(sql.contains("CREATE TABLE documents"));
        assert!(sql.contains("VECTOR(768)"));
    }
}
