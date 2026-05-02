# PardusDB Skill

Referencia rápida para agentes IA que interactúan con PardusDB.

## Repositorio

```
ssh://git@svr1.serinfo.com.es:6022/home/git/pardus-rag.git
```

## Estructura del Proyecto

```
pardus-rag/
├── src/lib.rs              # Librería Rust (API principal)
├── src/main.rs             # Binario CLI (REPL)
├── mcp/src/server.py       # Servidor MCP (Model Context Protocol)
├── sdk/python/             # SDK Python
├── sdk/typescript/         # SDK TypeScript
├── examples/               # Ejemplos de uso
└── setup.sh / install.sh   # Instaladores
```

## Instalación

```bash
git clone ssh://git@svr1.serinfo.com.es:6022/home/git/pardus-rag.git
cd pardus-rag
./setup.sh --install
```

Ver [INSTALL.md](INSTALL.md) para opciones de instalación detalladas.

## Uso del REPL

### Comandos Meta (dot-commands)

| Comando | Descripción |
|---------|-------------|
| `.create <file>` | Crear y abrir nueva base de datos |
| `.open <file>` | Abrir base de datos existente |
| `.save` | Forzar guardado |
| `.tables` | Listar tablas |
| `.clear` / `.cls` | Limpiar pantalla |
| `help` | Mostrar ayuda |
| `quit` / `exit` / `q` | Salir (guarda automáticamente si hay archivo abierto) |

### Uso

```bash
pardus                    # Abre ~/.pardus/pardus-rag.db (crea si no existe)
pardus mi.db              # Abre archivo específico
pardusdb                  # Sesión en memoria (sin persistencia)
pardusdb ~/.pardus/mi.db  # Abrir archivo específico
```

## Sintaxis SQL

### Tipos de Datos

| Tipo | Descripción | Ejemplo |
|------|-------------|---------|
| `VECTOR(n)` | Vector n-dimensional float | `VECTOR(768)` |
| `TEXT` | String UTF-8 | `'hello world'` |
| `INTEGER` | Entero 64-bit | `42` |
| `FLOAT` | Float 64-bit | `3.14` |
| `BOOLEAN` | true/false | `true` |

### Crear Tabla

```sql
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    embedding VECTOR(768),
    title TEXT,
    content TEXT,
    score FLOAT
);
```

### Insertar Datos

```sql
INSERT INTO documents (embedding, title, content, score)
VALUES ([0.1, 0.2, ...], 'Introduction to Rust', 'Content here', 0.95);
```

### Búsqueda por Similitud

```sql
SELECT * FROM documents
WHERE embedding SIMILARITY [0.12, 0.24, ...]
LIMIT 10;
```

Resultados ordenados por distancia (más cercano primero).

### Otras Operaciones

```sql
-- Seleccionar con filtro
SELECT * FROM documents WHERE title = 'Tutorial' LIMIT 10;

-- Actualizar
UPDATE documents SET score = 0.99 WHERE id = 1;

-- Eliminar
DELETE FROM documents WHERE id = 1;

-- Mostrar tablas
SHOW TABLES;

-- Eliminar tabla
DROP TABLE documents;
```

### UNIQUE Constraint

```sql
CREATE TABLE users (
    embedding VECTOR(128),
    id INTEGER PRIMARY KEY,
    email TEXT UNIQUE
);
```

### GROUP BY con Agregados

```sql
SELECT category, COUNT(*), AVG(score)
FROM documents
GROUP BY category;
```

### JOINs

```sql
SELECT * FROM orders
INNER JOIN users ON orders.user_id = users.id;
```

## API Rust

PardusDB tiene dos niveles de API:

### API de Alto Nivel (SQL)

```rust
use pardusdb::{Database, Value};

let mut db = Database::in_memory();

// Ejecutar SQL (CREATE, INSERT, SELECT, etc.)
db.execute("CREATE TABLE docs (embedding VECTOR(768), title TEXT)")?;
db.execute("INSERT INTO docs (embedding, title) VALUES ([0.1, 0.2, ...], 'Hello')")?;

// Consultar
let result = db.execute("SELECT * FROM docs LIMIT 10")?;
println!("{}", result);
```

### API de Bajo Nivel (Vectores)

```rust
use pardusdb::{VectorDB, EuclideanDB};

let db: EuclideanDB<f32> = VectorDB::in_memory(2);

db.insert(vec![0.0, 0.0])?;
db.insert(vec![1.0, 1.0])?;

// Búsqueda vectorial
let results = db.query(&[0.5, 0.5], 2)?;
```

## Python SDK

```bash
pip install -e sdk/python
```

```python
from pardusdb import PardusDB

client = PardusDB("mydb.pardus")
client.create_table("docs", vector_dim=768, metadata_schema={"title": "str", "content": "str"})
client.insert([0.1, 0.2, ...], metadata={"title": "Doc 1", "content": "Hello"})
results = client.search([0.1, 0.2, ...], k=5)
```

## Servidor MCP

Herramientas disponibles para agentes IA (OpenCode, Claude Desktop, etc.):

| Herramienta | Descripción |
|-------------|-------------|
| `pardusdb_create_database` | Crear archivo de base de datos |
| `pardusdb_open_database` | Abrir base de datos existente |
| `pardusdb_create_table` | Crear tabla con vectores y metadatos |
| `pardusdb_insert_vector` | Insertar un vector |
| `pardusdb_batch_insert` | Insertar múltiples vectores |
| `pardusdb_search_similar` | Buscar por similitud de vectores |
| `pardusdb_search_text` | Buscar por texto (genera embedding automáticamente) |
| `pardusdb_execute_sql` | Ejecutar SQL raw |
| `pardusdb_list_tables` | Listar tablas |
| `pardusdb_use_table` | Establecer tabla activa |
| `pardusdb_status` | Estado de la conexión |
| `pardusdb_import_text` | Importar documentos desde directorio |
| `pardusdb_health_check` | Verificar integridad de la base de datos |
| `pardusdb_get_schema` | Ver esquema de una tabla |
| `pardusdb_import_status` | Ver o reiniciar historial de importaciones |

### Configuración MCP en OpenCode

```json
{
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["python3", "/home/${USER}/.pardus/mcp/server.py"],
      "enabled": true
    }
  }
}
```

## Importación de Documentos

El servidor MCP puede importar documentos con embeddings automáticos:

**Formatos soportados:** PDF, CSV, DOCX, XLSX, XLS, JSON, JSONL, MD, TXT

**Estructura parent-child:** Archivos multipágina crean un registro padre + un hijo por página/párrafo.

**Ejemplo:**
```
Import all documents from /home/user/docs into a table called documents.
```

## Casos de Uso Comunes

### RAG (Retrieval-Augmented Generation)

```rust
use pardusdb::{Database, VectorDB, EuclideanDB};

// 1. Crear base de datos y tabla
let mut db = Database::in_memory();
db.execute("CREATE TABLE docs (embedding VECTOR(1536), content TEXT, source TEXT)")?;

// 2. Insertar documentos con embeddings generados externamente
// (usar sentence-transformers o similar para generar embeddings)
let embedding = generate_embedding("texto del documento");
db.execute("INSERT INTO docs (embedding, content) VALUES ([...], 'contenido')")?;

// 3. Buscar contexto relevante (usando API de bajo nivel)
let db_vec: EuclideanDB<f32> = VectorDB::in_memory(1536);
// Insertar vectores en el Graph para búsqueda
db_vec.insert(embedding)?;

let results = db_vec.query(&query_embedding, 5)?;
```

**O usando SQL directamente:**
```rust
// Crear tabla con dimensión adecuada
db.execute("CREATE TABLE docs (embedding VECTOR(384), content TEXT)")?;

// Insertar con embedding
db.execute("INSERT INTO docs (embedding, content) VALUES ([0.1, 0.2, ...], 'texto')")?;

// Buscar similaridad
let results = db.execute(
    "SELECT * FROM docs WHERE embedding SIMILARITY [0.1, 0.2, ...] LIMIT 5"
)?;
```

### Búsqueda Semántica

```sql
CREATE TABLE knowledge_base (
    embedding VECTOR(384),
    title TEXT,
    body TEXT
);

SELECT * FROM knowledge_base
WHERE embedding SIMILARITY [0.1, 0.2, ...]
LIMIT 20;
```

## Parámetros de Rendimiento

### Batch Inserts

Siempre usar inserts en batch para carga masiva:

| Batch Size | Speedup |
|------------|---------|
| Individual | 1x |
| 100 | 45x |
| 500 | 149x |
| 1000 | 220x |

### Búsqueda

- `k`: Número de resultados
- `ef_search`: Ancho de beam (mayor = más preciso, más lento)

### Dimensiones de Vectores

- Modelos sentence-transformers: 384 o 768
- OpenAI text-embedding-ada-002: 1536
- Todos los vectores en una tabla deben tener la misma dimensión

## Troubleshooting

### "Vector dimension mismatch"

Todos los vectores en una tabla deben tener la misma dimensión. Verificar que la tabla fue creada con el dimensión correcto.

### "Table not found"

- Tabla creada con `CREATE TABLE`
- Nombre correcto (case-sensitive)
- Conexión a base de datos válida

### Lentitud en inserts

Usar inserts en batch es más rápido, pero cada INSERT es una fila:

```sql
-- Un INSERT por fila (el parser no soporta multi-VALUE)
INSERT INTO docs (embedding, title) VALUES ([0.1, 0.2, ...], 'Doc 1');
INSERT INTO docs (embedding, title) VALUES ([0.3, 0.4, ...], 'Doc 2');
INSERT INTO docs (embedding, title) VALUES ([0.5, 0.6, ...], 'Doc 3');
```

**En código Rust:**
```rust
// Cada execute() es un INSERT individual
for doc in documents {
    db.execute(&format!(
        "INSERT INTO docs (embedding, content) VALUES ([{}], '{}')",
        doc.embedding.iter().join(", "),
        doc.content
    ))?;
}
```

### Lentitud en búsquedas

Ajustar parámetro `ef_search`:
- 50-100: Más rápido, menos preciso
- 200-500: Más preciso, más lento

## Recursos

- **Repositorio:** ssh://git@svr1.serinfo.com.es:6022/home/git/pardus-rag.git
- **Ejemplos:** Ver `examples/` en el repositorio
- **Python SDK Docs:** `sdk/python/README.md`
- **Instalación detallada:** [INSTALL.md](INSTALL.md)