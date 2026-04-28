# PardusDB: Base de Datos Vectorial Embebida

**Velocidad de SQLite, Potencia de Búsqueda Vectorial**

---

PardusDB es una base de datos vectorial embebida, monofichero, escrita en Rust.
Ofrece búsqueda de similitud vectorial con sintaxis SQL, persistencia tipo SQLite
y un servidor MCP para agentes de IA.

Está diseñada para desarrolladores que construyen aplicaciones de IA locales:
pipelines RAG, búsqueda semántica, sistemas de recomendación o cualquier proyecto
que necesite almacenamiento vectorial ligero sin depender de servicios externos.

---

## Características Principales

- **Almacenamiento monofichero** — Todo en un solo archivo `.pardus`, como SQLite.
- **Sintaxis SQL** — CREATE, INSERT, SELECT, UPDATE, DELETE con soporte para
  GROUP BY, HAVING, JOINs, subconsultas y condiciones compuestas.
- **Búsqueda vectorial HNSW** — Aproximada, graph-based, con métricas Cosine,
  Dot Product y Euclidean.
- **Hilos seguros** — Lecturas concurrentes seguras en aplicaciones multihilo.
- **Transacciones completas** — BEGIN/COMMIT/ROLLBACK para operaciones atómicas.
- **Aceleración GPU opcional** — Para inserciones y consultas por lote grandes.
- **Importación de documentos** — PDF, CSV, DOCX, XLSX, JSON, JSONL, MD, TXT con
  embeddings automáticos y seguimiento padre-hijo.
- **Servidor MCP** — Permite que agentes de IA (OpenCode, Claude) interactúen
  con la base de datos directamente.
- **SDKs** — Python y TypeScript.

---

## Rendimiento

Comparativas con bases de datos vectoriales establecidas:

| vs Neo4j | Ventaja PardusDB |
|----------|-----------------|
| Inserción | **1.983× más rápida** |
| Búsqueda | **431× más rápida** |

| vs HelixDB | Ventaja PardusDB |
|------------|-----------------|
| Inserción | **200× más rápida** |
| Búsqueda | **62× más rápida** |

Inserción por lotes — el lote de 1000 documentos es **220× más rápido** que
insertar uno por uno.

---

## Instalación en 30 Segundos

```bash
git clone https://github.com/FErArg/PardusDB
cd PardusDB
./setup.sh --install
```

También disponible con binario precompilado (sin necesidad de Rust):

```bash
./install.sh --install
```

---

## Ejemplo de Uso

```sql
CREATE TABLE documentos (
    embedding VECTOR(768),
    titulo TEXT,
    contenido TEXT
);

INSERT INTO documentos (embedding, titulo, contenido)
VALUES ([0.12, 0.34, ...], 'Introducción', 'Texto de ejemplo');

SELECT * FROM documentos
WHERE embedding SIMILARITY [0.12, 0.34, ...]
LIMIT 5;
```

---

## Enlaces

- **Repositorio activo (fork):** [github.com/FErArg/PardusDB](https://github.com/FErArg/PardusDB)
- **Proyecto original:** [github.com/JasonHonKL/PardusDB](https://github.com/JasonHonKL/PardusDB)

---

*Licencia MIT — uso libre en proyectos personales y comerciales.*
