"""
LightRAG Setup - Knowledge Graph + Vector RAG System
Использует граф-базы данных (Neo4j, NetworkX) + векторные (Milvus, Chroma, FAISS)

Установка:
    pip install lightrag-hku tiktoken nano_vectordb
    pip install "lightrag-hku[api]"  # Для REST API

Для Neo4j:
    pip install neo4j

Для FAISS:
    pip install faiss-cpu  # или faiss-gpu

Для Chroma:
    pip install chromadb

Для Milvus:
    pip install pymilvus

Документация: https://github.com/HKUDS/LightRAG
"""

import os


# Check dependencies
def check_dependencies():
    """Check which dependencies are installed."""
    deps = {
        "lightrag": False,
        "tiktoken": False,
        "nano_vectordb": False,
        "neo4j": False,
        "faiss": False,
        "chromadb": False,
        "pymilvus": False,
    }

    try:
        import lightrag  # noqa: F401

        deps["lightrag"] = True
    except ImportError:
        pass

    try:
        import tiktoken  # noqa: F401

        deps["tiktoken"] = True
    except ImportError:
        pass

    try:
        import nano_vectordb  # noqa: F401

        deps["nano_vectordb"] = True
    except ImportError:
        pass

    try:
        import neo4j  # noqa: F401

        deps["neo4j"] = True
    except ImportError:
        pass

    try:
        import faiss  # noqa: F401

        deps["faiss"] = True
    except ImportError:
        pass

    try:
        import chromadb  # noqa: F401

        deps["chromadb"] = True
    except ImportError:
        pass

    try:
        import pymilvus  # noqa: F401

        deps["pymilvus"] = True
    except ImportError:
        pass

    return deps


def print_installation_guide():
    """Print installation guide."""
    deps = check_dependencies()

    print("╔═══════════════════════════════════════════════════════════════╗")
    print("║  LightRAG - Knowledge Graph + Vector RAG                      ║")
    print("╚═══════════════════════════════════════════════════════════════╝")

    print("\n📦 Статус зависимостей:")
    for dep, installed in deps.items():
        status = "✓" if installed else "✗"
        print(f"   {status} {dep}")

    print("\n📥 Команды установки:")
    print("   # Базовая установка")
    print("   pip install lightrag-hku tiktoken nano_vectordb")
    print("")
    print("   # С REST API")
    print("   pip install 'lightrag-hku[api]'")
    print("")
    print("   # Векторные базы данных:")
    print("   pip install faiss-cpu        # FAISS (CPU)")
    print("   pip install faiss-gpu        # FAISS (GPU)")
    print("   pip install chromadb         # Chroma")
    print("   pip install pymilvus         # Milvus")
    print("")
    print("   # Граф-базы данных:")
    print("   pip install neo4j            # Neo4j")

    print("\n🗄️ Поддерживаемые базы данных:")
    print("   Векторные: NanoVectorDB, FAISS, Chroma, Milvus, PGVector, MongoDB, Qdrant")
    print("   Графовые:  NetworkX (встроен), Neo4j, PostgreSQL+AGE")

    return deps


async def setup_lightrag(
    working_dir: str = "./lightrag_data",
    use_openai: bool = True,
    use_ollama: bool = False,
    ollama_model: str = "qwen2.5:7b",
    vector_storage: str = "nano",  # nano, faiss, chroma, milvus
    graph_storage: str = "networkx",  # networkx, neo4j
):
    """
    Initialize LightRAG with specified backends.

    Args:
        working_dir: Directory for data storage
        use_openai: Use OpenAI for LLM and embeddings
        use_ollama: Use Ollama for local LLM
        ollama_model: Ollama model name
        vector_storage: Vector DB type
        graph_storage: Graph DB type

    Returns:
        Configured LightRAG instance
    """
    try:
        from lightrag import LightRAG, QueryParam  # noqa: F401
        from lightrag.kg.shared_storage import initialize_pipeline_status  # noqa: F401
    except ImportError:
        print("❌ LightRAG не установлен!")
        print("   pip install lightrag-hku tiktoken nano_vectordb")
        return None

    os.makedirs(working_dir, exist_ok=True)

    # Configure LLM
    if use_openai:
        from lightrag.llm.openai import gpt_4o_mini_complete, openai_embed

        llm_func = gpt_4o_mini_complete
        embed_func = openai_embed
        print("✓ Используем OpenAI API")

    elif use_ollama:
        from lightrag.llm.ollama import ollama_model_complete, ollama_embed

        def llm_func(prompt, **kwargs):
            return ollama_model_complete(prompt, model_name=ollama_model, **kwargs)

        embed_func = ollama_embed
        print(f"✓ Используем Ollama ({ollama_model})")

    else:
        print("❌ Нужно выбрать OpenAI или Ollama")
        return None

    # Configure storage
    storage_config = {}

    if vector_storage == "nano":
        # Default NanoVectorDB (file-based)
        pass
    elif vector_storage == "faiss":
        try:
            from lightrag.kg.faiss_impl import FaissVectorDBStorage

            storage_config["vector_storage"] = FaissVectorDBStorage
        except ImportError:
            print("⚠ FAISS не установлен, используем NanoVectorDB")
    elif vector_storage == "chroma":
        try:
            from lightrag.kg.chroma_impl import ChromaVectorDBStorage

            storage_config["vector_storage"] = ChromaVectorDBStorage
        except ImportError:
            print("⚠ ChromaDB не установлен, используем NanoVectorDB")

    if graph_storage == "neo4j":
        try:
            from lightrag.kg.neo4j_impl import Neo4JStorage

            storage_config["graph_storage"] = Neo4JStorage
            # Need NEO4J_URI, NEO4J_USER, NEO4J_PASSWORD env vars
            print("✓ Используем Neo4j")
        except ImportError:
            print("⚠ Neo4j не установлен, используем NetworkX")

    # Create RAG instance
    rag = LightRAG(working_dir=working_dir, embedding_func=embed_func, llm_model_func=llm_func, **storage_config)

    # Initialize
    await rag.initialize_storages()
    await initialize_pipeline_status()

    print(f"✓ LightRAG инициализирован в {working_dir}")
    return rag


async def demo():
    """Demo usage of LightRAG."""
    deps = check_dependencies()

    if not deps["lightrag"]:
        print_installation_guide()
        return

    print("\n🚀 Демонстрация LightRAG")
    print("=" * 50)

    # Initialize
    rag = await setup_lightrag(
        working_dir="./demo_lightrag",
        use_openai=True,
    )

    if not rag:
        return

    # Sample document
    sample_text = """
    Центр Пример (example.org) - это образовательное пространство в тестовом городе.
    Центр проводит мероприятия: медитации, йога-классы, семинары.
    Команда состоит из преподавателей с опытом практики более 20 лет.
    Центр расположен в историческом здании в центре города.
    Регулярные встречи проходят по выходным.
    """

    # Insert document
    print("\n📄 Добавляем документ...")
    await rag.ainsert(sample_text)
    print("✓ Документ добавлен")

    # Query modes
    from lightrag import QueryParam

    modes = ["naive", "local", "global", "hybrid", "mix"]

    print("\n🔍 Тестируем разные режимы поиска:")
    for mode in modes:
        try:
            result = await rag.aquery("Что такое Центр Пример?", param=QueryParam(mode=mode))
            print(f"\n   [{mode}]: {result[:200]}...")
        except Exception as e:
            print(f"\n   [{mode}]: Ошибка - {e}")


if __name__ == "__main__":
    print_installation_guide()

    print("\n" + "=" * 60)
    print("Для запуска демо:")
    print("   python -c 'import asyncio; from lightrag_setup import demo; asyncio.run(demo())'")
