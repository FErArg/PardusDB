# PardusDB MCP — Guía para Agentes AI

## Configuración

Agregar en el archivo de configuración MCP del proyecto (e.g., `opencode.json`):

```json
{
  "mcp": {
    "pardusdb": {
      "type": "local",
      "command": ["/path/to/mcp/run_pardusdb_mcp.sh"],
      "enabled": true
    }
  }
}
```

**Nota:** Ajustar la ruta al `server.py` según la instalación del proyecto.

---

## Herramientas Disponibles (15)

### Gestión de Base de Datos

| Herramienta | Descripción | Parámetros requeridos |
|------------|-------------|----------------------|
| `pardusdb_create_database` | Crear nuevo archivo `.pardus` | `path` |
| `pardusdb_open_database` | Abrir base de datos existente | `path` |
| `pardusdb_status` | Ver estado actual de conexión | — |

### Gestión de Tablas

| Herramienta | Descripción | Parámetros requeridos |
|------------|-------------|----------------------|
| `pardusdb_create_table` | Crear tabla para vectores | `name`, `vector_dim` |
| `pardusdb_list_tables` | Listar todas las tablas | — |
| `pardusdb_use_table` | Establecer tabla activa | `table` |
| `pardusdb_get_schema` | Ver esquema y estadísticas | `table` |

### Inserción de Vectores

| Herramienta | Descripción | Parámetros requeridos |
|------------|-------------|----------------------|
| `pardusdb_insert_vector` | Insertar vector individual | `vector` |
| `pardusdb_batch_insert` | Insertar múltiples vectores | `vectors` |
| `pardusdb_execute_sql` | Ejecutar comando SQL raw | `sql` |

### Búsqueda

| Herramienta | Descripción | Parámetros requeridos |
|------------|-------------|----------------------|
| `pardusdb_search_similar` | Buscar por vector (cosine similarity) | `query_vector` |
| `pardusdb_search_text` | Buscar por texto con auto-embeddings | `query` |

### Importación de Documentos

| Herramienta | Descripción | Parámetros requeridos |
|------------|-------------|----------------------|
| `pardusdb_import_text` | Importar PDFs, CSVs, DOCX, XLSX, JSON, MD, TXT | `dir_path`, `table` |
| `pardusdb_import_status` | Ver o resetear historial de imports | `action` |

### Mantenimiento

| Herramienta | Descripción | Parámetros requeridos |
|------------|-------------|----------------------|
| `pardusdb_health_check` | Verificar integridad de la base | `table` (opcional) |

---

## Workflows Típicos

### Nuevo proyecto desde cero

```
1. pardusdb_create_database(path="/mi/proyecto/data.pardus")
2. pardusdb_create_table(name="documentos", vector_dim=384)
3. pardusdb_import_text(dir_path="/data/documentos", table="documentos")
```

### Abrir proyecto existente y buscar

```
1. pardusdb_open_database(path="/mi/proyecto/data.pardus")
2. pardusdb_search_text(query="consulta del usuario", k=10)
```

### Crear tabla con metadatos personalizados

```
1. pardusdb_create_table(
     name="articulos",
     vector_dim=768,
     metadata_schema={"titulo": "str", "categoria": "str", "precio": "float"}
   )
```

### Insertar vectores manualmente

```
// Individual
pardusdb_insert_vector(
  vector=[0.1, 0.2, 0.3, ...],
  metadata={"titulo": "Artículo 1", "categoria": "tech"},
  table="articulos"
)

// Batch
pardusdb_batch_insert(
  vectors=[[0.1, 0.2, ...], [0.4, 0.5, ...]],
  metadata_list=[{"titulo": "A"}, {"titulo": "B"}],
  table="articulos"
)
```

### Buscar con SQL raw

```
pardusdb_execute_sql(sql="SELECT * FROM articulos WHERE categoria = 'tech' LIMIT 10")
```

---

## Opciones de import_text

```python
pardusdb_import_text(
  dir_path="/data",              # Directorio a escanear (requerido)
  table="documentos",            # Tabla destino (requerido)
  file_patterns=[".pdf", ".md"], # Tipos de archivo (opcional, default: todos)
  recursive=true,               # Buscar en subdirectorios (default: true)
  max_file_size_mb=50,          # Tamaño máximo por archivo (default: 50)
  vector_dim=384                 # Dimensión de embeddings (default: 384)
)
```

**Formatos soportados:** `.pdf`, `.csv`, `.docx`, `.xlsx`, `.json`, `.jsonl`, `.md`, `.txt`

**Notas:**
- Archivos multi-página crean registros padre + hijos con tracking
- Archivos ya importados se skippean (por SHA256 hash)
- Embeddings generados con `all-MiniLM-L6-v2` por defecto

---

## Notas Técnicas

| Aspecto | Detalle |
|---------|---------|
| Timeout | 60 segundos por operación |
| Formato vector | Array de floats (e.g., `[0.1, 0.2, 0.3]`) |
| Embedding default | `all-MiniLM-L6-v2` (384 dim) |
| Modelos embedding | Todos los de `sentence-transformers` |

---

## Troubleshooting

### "Database file not found"
- Verificar que el path existe
- Usar `pardusdb_create_database` si el archivo no existe

### "Query timed out"
- Archivos grandes tardan más en cargar
- El timeout actual es 60s

### "Table must have a VECTOR column"
- Al crear tabla, siempre especificar `vector_dim`

### "No database opened"
- Ejecutar `pardusdb_open_database` antes de cualquier operación

---

## Ejemplo Completo: RAG Pipeline

```python
# 1. Crear base de datos
pardusdb_create_database(path="./mi_rag.pardus")

# 2. Crear tabla con vectores de 384 dimensiones
pardusdb_create_table(name="documentos", vector_dim=384)

# 3. Importar documentos desde un directorio
pardusdb_import_text(
  dir_path="./documentos",
  table="documentos",
  file_patterns=[".pdf", ".md", ".txt"]
)

# 4. Cuando el usuario hace una pregunta...
pardusdb_search_text(query="¿Qué dice el documento sobre X?", k=5)

# 5. Usar resultados para contexto en el LLM
```