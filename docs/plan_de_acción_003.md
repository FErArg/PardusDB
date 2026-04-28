# Roadmap — Funcionalidades Propuestas

Priorizadas por impacto/esfuerzo. El orden recomendado de implementación es ascendente.

---

## 1. Batch insert multi-row en MCP server

**Esfuerzo:** Muy pequeño (~10 líneas, solo Python)
**Impacto:** ⭐⭐⭐
**Archivos:** `mcp/src/server.py`

**Problema:** `handle_batch_insert()` itera vectores y ejecuta `INSERT INTO` individual por cada uno, spawnendo un subproceso `pardusdb` por iteración.

**Solución:** Cambiar `handle_batch_insert()` para generar una sola sentencia SQL multi-row:
```sql
INSERT INTO table (embedding, ...) VALUES ([...], ...), ([...], ...), ...
```
El parser SQL de `pardusdb` ya soporta multi-row (v0.4.12). Solo el MCP server no lo usa.

---

## 2. `AS` alias en columnas simples del parser SQL

**Esfuerzo:** Pequeño (~30 líneas, solo Rust)
**Impacto:** ⭐⭐⭐
**Archivos:** `src/parser.rs`, `src/database.rs`, `src/concurrent.rs`
**Plan detallado:** `docs/parser-alias-groupby-plan.md`

**Problema:** `SelectColumn::Column(String)` no tiene campo `alias`. `SELECT title AS t FROM docs` falla.

**Solución:** Cambiar a `SelectColumn::Column(String, Option<String>)`, añadir parseo de `AS` tras identificadores, propagar alias en `database.rs` para display y resolución en `ORDER BY`/`GROUP BY`/`HAVING`.

---

## 3. `IF NOT EXISTS` en CREATE TABLE

**Esfuerzo:** Muy pequeño (~5 líneas, solo Rust)
**Impacto:** ⭐⭐
**Archivos:** `src/parser.rs`, `src/database.rs`

**Problema:** `CREATE TABLE` no soporta `IF NOT EXISTS`. El MCP server tiene que hacer workarounds frágiles (SELECT LIMIT 1) para detectar existencia de tabla.

**Solución:** Añadir parseo de `IF NOT EXISTS` tras `CREATE TABLE`, y en `database.rs` retornar `Ok` en vez de error si la tabla ya existe y el flag está presente.

---

## 4. `DESCRIBE TABLE` / Schema query

**Esfuerzo:** Pequeño (~20 líneas, Rust parser + database)
**Impacto:** ⭐⭐⭐
**Archivos:** `src/parser.rs`, `src/database.rs`

**Problema:** No hay forma de inspeccionar el esquema de una tabla vía SQL. `SHOW TABLES` solo lista nombres. El MCP server admite esta limitación.

**Solución:** Añadir comando `DESCRIBE <table>` (o `SHOW COLUMNS FROM <table>`) al parser. `database.rs` devuelve columnas, tipos, constraints.

---

## 5. Conexión persistente en MCP server

**Esfuerzo:** Medio (~50 líneas, solo Python)
**Impacto:** ⭐⭐⭐⭐⭐
**Archivos:** `mcp/src/server.py`

**Problema:** Cada llamada MCP spawnerea `subprocess.run(["pardusdb", ...])`, que abre archivo, ejecuta SQL, hace `save()`, y muere. Inserciones múltiples son ~220x más lentas que batch porque el overhead de spawn domina.

**Solución:** `PardusDBClient.execute()` mantiene el subproceso abierto. Envía comandos por stdin, lee resultados por stdout. Solo spawnrea una vez. Enviar `quit` cierra la conexión. Opcional: reconexión automática si el proceso muere.

**Riesgo:** El proceso `pardusdb` corriendo como daemon debe responder correctamente a comandos secuenciales por stdin. Actualmente `run_with_file()` ya lee stdin línea por línea, así que debería funcionar sin cambios en Rust.

---

## 6. HTTP server con long-lived process

**Esfuerzo:** Pequeño (~120 líneas, solo Python)
**Impacto:** ⭐⭐⭐⭐
**Archivos:** `mcp/src/http_server.py` (nuevo), `mcp/src/server.py` (+5 líneas)
**Plan detallado:** `docs/http-mcp-server-plan.md`

**Problema:** El MCP server solo funciona vía stdio (para OpenCode/Claude). No hay forma de acceder desde scripts, curl, o aplicaciones web.

**Solución:** Servidor HTTP en localhost:16121 que reusa todas las `handle_*` functions de server.py. Cada endpoint POST mapea 1:1 a un tool MCP.

**Nota:** El HTTP server spawnerea un subproceso `pardusdb` por request si no se implementa #5 primero. Combinado con #5, el HTTP server comparte una conexión persistente y es mucho más rápido.

---

## 7. Output JSON en binario `pardusdb`

**Esfuerzo:** Medio (~80 líneas, Rust)
**Impacto:** ⭐⭐⭐⭐
**Archivos:** `src/main.rs`, `src/database.rs`

**Problema:** El binario `pardusdb` siempre devuelve texto plano formateado para humanos. El MCP server parsea con regex (`parse_id_from_result`, `re.search(r"Count:\s*(\d+)")`), lo cual es frágil.

**Solución:** Añadir flag `--json` que hace que `run_with_file()` o `run_repl()` devuelvan resultados en JSON estructurado.

---

## 8. Streaming de importación con progreso

**Esfuerzo:** Grande (~150 líneas Rust + Python)
**Impacto:** ⭐⭐
**Archivos:** `src/main.rs`, `mcp/src/server.py`

**Problema:** `handle_import_text()` procesa cientos de archivos sin feedback hasta el final. El usuario no sabe si el proceso avanzó o colgó.

**Solución:** El binario `pardusdb` reporta progreso en stderr con formato `{"progress": {"current": 5, "total": 100}}`. El MCP server puede leer stderr y exponer el progreso.

---

## Resumen

| # | Funcionalidad | Archivos | Esfuerzo | Impacto |
|---|---------------|----------|----------|---------|
| 1 | Batch insert multi-row en MCP | `server.py` | Muy pequeño | ⭐⭐⭐ |
| 2 | `AS` alias en columnas | Rust parser + database | Pequeño | ⭐⭐⭐ |
| 3 | `IF NOT EXISTS` | Rust parser + database | Muy pequeño | ⭐⭐ |
| 4 | `DESCRIBE TABLE` | Rust parser + database | Pequeño | ⭐⭐⭐ |
| 5 | Conexión persistente MCP | `server.py` | Medio | ⭐⭐⭐⭐⭐ |
| 6 | HTTP server | `http_server.py` nuevo | Pequeño | ⭐⭐⭐⭐ |
| 7 | Output JSON en binario | Rust main + database | Medio | ⭐⭐⭐⭐ |
| 8 | Streaming importación | Rust + Python | Grande | ⭐⭐ |
