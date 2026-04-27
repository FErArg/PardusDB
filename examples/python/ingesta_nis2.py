#!/usr/bin/env python3
"""
Ingesta los documentos NIS2 en PardusDB usando la SDK Python.
Documentos únicos:
  1. CELEX_32022L2555_ES_TXT -> Directiva NIS2 oficial (completa) ~379KB
  2. Directiva NIS2 -> Guía de cumplimiento ~3.5KB
  3. 02-INFO_NIS2-Ámbito Aplicación -> Ámbito de aplicación ~4.4KB
"""

import hashlib
import re
import unicodedata

from sentence_transformers import SentenceTransformer
from pardusdb import PardusDB

# === CONFIG ===
DB_PATH = "/home/ferarg/Nextcloud/Documentos/IA/pardus-demo/pardus-demo.pardus"
CHUNK_SIZE = 512
CHUNK_OVERLAP = 64
EMBED_MODEL = "all-MiniLM-L6-v2"

DOCS = [
    {
        "path": "/tmp/CELEX_32022L2555_ES_TXT.txt",
        "title": "Directiva NIS2 - Texto Oficial Completo",
        "source": "CELEX_32022L2555_ES_TXT",
        "type": "directiva_oficial",
    },
    {
        "path": "/tmp/Directiva NIS2.txt",
        "title": "Directiva NIS2 - Guía de Cumplimiento",
        "source": "Directiva_NIS2_Guia",
        "type": "guia_cumplimiento",
    },
    {
        "path": "/tmp/02-INFO_NIS2-Ámbito Aplicación.txt",
        "title": "Directiva NIS2 - Ámbito de Aplicación",
        "source": "INFO_NIS2_Ambito",
        "type": "infografia_ambito",
    },
]


def normalize_text(text: str) -> str:
    text = unicodedata.normalize("NFKC", text)
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def chunk_text(text: str, chunk_size: int = CHUNK_SIZE, overlap: int = CHUNK_OVERLAP) -> list[str]:
    words = text.split()
    chunks = []
    start = 0
    while start < len(words):
        end = min(start + chunk_size, len(words))
        chunk = " ".join(words[start:end])
        if chunk:
            chunks.append(chunk)
        if end == len(words):
            break
        start = end - overlap
    return chunks


def main():
    print("=== INGESTA NIS2 DOCS ===")

    print("[1/4] Cargando modelo de embeddings...")
    model = SentenceTransformer(EMBED_MODEL)
    dim = model.get_embedding_dimension()
    print(f"  Modelo: {EMBED_MODEL}, dimensión: {dim}")

    print("[2/4] Abriendo PardusDB...")
    db = PardusDB(DB_PATH)

    tables = db.list_tables()
    print(f"  Tablas existentes: {tables}")

    table_name = "nis2_docs"

    if table_name not in tables:
        print(f"  Creando tabla '{table_name}'...")
        db.create_table(
            name=table_name,
            vector_dim=dim,
            metadata_schema={
                "chunk_text": "str",
                "chunk_index": "int",
                "total_chunks": "int",
                "title": "str",
                "source": "str",
                "doc_type": "str",
                "file_hash": "str",
            },
        )
    else:
        print(f"  Tabla '{table_name}' ya existe")

    total_inserted = 0
    total_skipped = 0

    for doc in DOCS:
        path = doc["path"]
        title = doc["title"]
        source = doc["source"]
        doc_type = doc["type"]

        print(f"\n[3/4] Procesando: {title}")

        with open(path, "r", encoding="utf-8", errors="replace") as f:
            raw = f.read()
        text = normalize_text(raw)

        if not text or len(text) < 10:
            print(f"  [!] Vacío o demasiado corto, se omite")
            continue

        file_hash = hashlib.sha256(text.encode()).hexdigest()

        # Verificar duplicado
        result = db.raw_sql(
            f"SELECT COUNT(*) as cnt FROM {table_name} WHERE file_hash = '{file_hash}'"
        )
        if "1" in result.split():
            existing = True
        else:
            existing = bool(
                db.raw_sql(
                    f"SELECT COUNT(*) as cnt FROM {table_name} WHERE file_hash = '{file_hash}'"
                ).strip()
                != "0"
            )
        # Try more robust duplicate check
        raw_count = db.raw_sql(f"SELECT COUNT(*) FROM {table_name} WHERE file_hash = '{file_hash}'")
        try:
            dup_count = int(raw_count.strip())
        except ValueError:
            dup_count = 0

        if dup_count > 0:
            print(f"  [-] Ya importado (hash: {file_hash[:12]}...), se omite")
            total_skipped += 1
            continue

        # Limpiar y chunking
        texto_limpio = re.sub(r"[^\w\sáéíóúüñÑÁÉÍÓÚÜ,.;:()\-]", " ", text)
        texto_limpio = re.sub(r"\s+", " ", texto_limpio).strip()
        chunks = chunk_text(texto_limpio)
        n_chunks = len(chunks)
        print(f"  Chunks: {n_chunks}")

        if n_chunks == 0:
            continue

        print(f"  Generando embeddings...")
        embeddings = model.encode(chunks, show_progress_bar=True, normalize_embeddings=True)

        print(f"  Insertando en BD...")
        vectors = [emb.tolist() for emb in embeddings]
        metadata_list = [
            {
                "chunk_text": chunk_text,
                "chunk_index": i,
                "total_chunks": n_chunks,
                "title": title,
                "source": source,
                "doc_type": doc_type,
                "file_hash": file_hash,
            }
            for i, chunk_text in enumerate(chunks)
        ]

        ids = db.insert_batch(vectors=vectors, metadata_list=metadata_list, table=table_name)
        total_inserted += len(ids)
        print(f"  [+] Insertados {len(ids)} chunks (ids: {ids[0]}..{ids[-1]})")

    print(f"\n=== RESUMEN ===")
    print(f"  Insertados: {total_inserted} chunks")
    print(f"  Omitidos (duplicados): {total_skipped} documentos")

    # Stats
    print("\n--- Estadísticas por documento ---")
    for doc in DOCS:
        src = doc["source"]
        result = db.raw_sql(
            f"SELECT COUNT(*) FROM {table_name} WHERE source = '{src}'"
        )
        print(f"  {doc['title'][:50]:50s} -> {result.strip()} chunks")


if __name__ == "__main__":
    main()
