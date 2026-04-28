# Plan: Expose MCP via HTTP en `localhost:16121`

## Arquitectura Actual

El MCP server (`mcp/src/server.py`) usa transporte `stdio` del paquete `mcp` Python. Comunicación JSON-RPC sobre stdin/stdout. Diseñado para agentes AI (OpenCode, Claude Desktop) que lo lanzan como subproceso.

La lógica de negocio está en 15 funciones `handle_*` asincrónicas, todas usando el singleton `db_client = PardusDBClient()` para ejecutar SQL vía el binario `pardusdb`.

## Objetivo

Exponer las mismas 15 operaciones vía HTTP en `localhost:16121`, manteniendo el stdio MCP funcionando. Ambos modos deben poder ejecutarse simultáneamente como procesos separados.

## Enfoque

Crear `mcp/src/http_server.py` que importa las funciones `handle_*` y `db_client` desde `server.py`. Modificar `server.py` mínimamente para detectar flag `--http`.

## Archivos

### Nuevo: `mcp/src/http_server.py`

Servidor HTTP con 15 rutas POST + 1 GET. Reusa todas las funciones existentes.

```python
ROUTES = {
    "POST": {
        "/api/create_database": handle_create_database,
        "/api/open_database": handle_open_database,
        "/api/create_table": handle_create_table,
        "/api/insert_vector": handle_insert_vector,
        "/api/batch_insert": handle_batch_insert,
        "/api/search_similar": handle_search_similar,
        "/api/search_text": handle_search_text,
        "/api/execute_sql": handle_execute_sql,
        "/api/list_tables": handle_list_tables,
        "/api/use_table": handle_use_table,
        "/api/import_text": handle_import_text,
        "/api/health_check": handle_health_check,
        "/api/get_schema": handle_get_schema,
        "/api/import_status": handle_import_status,
    },
    "GET": {
        "/api/status": handle_get_status,
    }
}
```

Usa `ThreadingHTTPServer` (stdlib, Python 3.7+) para manejar requests concurrentes sin depender de asyncio. Las funciones `handle_*` son async, pero internamente son síncronas (solo llaman a `db_client.execute()`). Se llaman con `asyncio.run()` por request.

**Formato de respuesta:**
```json
{"success": true, "data": "texto del resultado"}
```

**Binding:** `127.0.0.1:16121` (localhost solamente).

### Modificado: `mcp/src/server.py` (+5 líneas al final)

```python
if __name__ == "__main__":
    if "--http" in sys.argv:
        from http_server import run
        port = 16121
        for i, arg in enumerate(sys.argv):
            if arg == "--http-port" and i + 1 < len(sys.argv):
                port = int(sys.argv[i + 1])
        run(port=port)
    else:
        # Existing stdio MCP behavior
        asyncio.run(main())
```

**Uso:**
```bash
python3 mcp/src/server.py                   # stdio MCP (comportamiento actual)
python3 mcp/src/server.py --http            # HTTP en :16121
python3 mcp/src/server.py --http --http-port 9090  # HTTP en :9090
```

## Modos Duales

No pueden correr en el mismo proceso (uno bloquea el event loop). Se lanzan como procesos separados:

```bash
# Terminal 1
python3 mcp/src/server.py

# Terminal 2
python3 mcp/src/server.py --http
```

Cada proceso tiene su propio `db_client` singleton y su propio subproceso `pardusdb`. No hay conflicto de estado.

## Problemas Potenciales

| # | Problema | Severidad | Mitigación |
|---|----------|-----------|------------|
| 1 | `asyncio.run()` desde handler síncrono crea event loop por request | **Media** | Las handlers son async pero en la práctica ejecutan todo síncrono (solo llaman `db_client.execute()`). Refactorizar handlers a síncronas elimina el problema. |
| 2 | `ThreadingHTTPServer` crea un thread por request concurrente | **Baja** | El número de requests concurrentes es bajo (uso local). ThreadPool limitado implícitamente. |
| 3 | `import_text` puede tardar minutos, bloquea un thread | **Media** | Es una operación larga. El thread se libera al terminar. No hay otros endpoints bloqueados porque cada request va a su thread. |
| 4 | Sin autenticación en HTTP | **Baja** | Bind a 127.0.0.1 solamente. Documentar que es local-only. |
| 5 | Sin API docs/swagger | **Baja** | No necesario para uso local. Si se necesita después, se puede añadir. |
| 6 | Puerto ocupado | **Baja** | Error claro al iniciar si el puerto está en uso. `--http-port` para cambiar. |

## Resumen de Cambios

| Archivo | Acción | Líneas |
|---------|--------|--------|
| `mcp/src/http_server.py` | Crear | ~120 |
| `mcp/src/server.py` | Modificar | +5 |
| Dependencias nuevas | No | stdlib solamente |
