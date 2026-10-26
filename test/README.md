# PardusDB Test Suite

Security and functionality tests for PardusDB.

## Estructura

```
test/
├── README.md           # Este archivo
├── pytest.ini         # Configuración pytest
├── conftest.py        # Fixtures compartidos
├── security_test.py   # Script standalone (ejecutable)
└── test_security.py   # Tests pytest
```

## Requisitos

- `pardusdb` en PATH
- Python 3.10+
- (Opcional) `pytest` para tests formales

```bash
pip install pytest
```

## Uso

### Script Standalone (sin pytest)

```bash
python3 test/security_test.py
```

Ejecuta todos los tests de seguridad y funcionalidad sin dependencias adicionales.

### Tests con pytest

```bash
# Todos los tests
pytest test/

# Solo security tests
pytest test/test_security.py -v

# Con output detallado
pytest test/test_security.py -v --tb=long
```

## Tests Incluidos

### Seguridad

| Test | Descripción |
|------|-------------|
| SQL Injection (OR) | Verifica que payloads `' OR '1'='1'` sean bloqueados |
| SQL Injection (DROP) | Verifica comportamiento de DROP TABLE |
| Path Traversal | Verifica que paths con `..` sean manejados |
| Vector Dimension | Verifica que vectores con dimensión incorrecta sean rechazados |

### Funcionalidad

| Test | Descripción |
|------|-------------|
| Create Table | Verifica que CREATE TABLE funcione |
| List Tables | Verifica que .tables muestre las tablas |
| Table Persistence | Verifica que tablas persistan después de save |
| Empty Table Query | Verifica que SELECT en tabla vacía no falle |

## Troubleshooting

### "BINARY_NOT_FOUND"

`pardusdb` no está en PATH. Asegúrate de que esté instalado:

```bash
which pardusdb
# o
pip install pardusdb  # si usaste el SDK
```

### "TIMEOUT"

El comando tardó más de 30s. Verifica que la base de datos no esté corrupta.

### Tests fallan con "dimension mismatch"

Esto es **esperado** para algunos tests de seguridad. Verifica que el mensaje contiene "dimension mismatch".

## Agregar Nuevos Tests

### Script standalone

Agregar funciones al final de `security_test.py`:

```python
def test_nuevo():
    print("TEST: Nuevo Test")
    # ... test logic ...
    print("  ✓ Pass")
```

### Pytest

Agregar en `test_security.py`:

```python
def test_nuevo(setup_db):
    """Description"""
    result = run_cmd("SQL COMMAND", setup_db)
    assert "expected" in result.lower()
```

## Notas

- Los tests usan bases de datos temporales que se eliminan automáticamente
- Timeout por comando: 30 segundos
- Compatible con Linux y macOS
