"""Tests for Kurigram client integration."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from integrations.kurigram_client import (
    Entity,
    GraphData,
    KurigramClient,
    KurigramConfig,
    KurigramError,
    Query,
    QueryResult,
    Relationship,
)


class TestKurigramConfig:
    """Test KurigramConfig class."""

    def test_from_env(self, monkeypatch):
        """Test loading config from environment."""
        monkeypatch.setenv("KURIGRAM_API_URL", "https://test.kurigram.com")
        monkeypatch.setenv("KURIGRAM_API_KEY", "test_api_key")

        config = KurigramConfig.from_env()

        assert config.api_url == "https://test.kurigram.com"
        assert config.api_key == "test_api_key"

    def test_from_env_with_defaults(self, monkeypatch):
        """Test loading config with default values."""
        monkeypatch.delenv("KURIGRAM_API_URL", raising=False)
        monkeypatch.setenv("KURIGRAM_API_KEY", "test_key")

        config = KurigramConfig.from_env()

        assert config.api_url == "http://localhost:8000"
        assert config.api_key == "test_key"

    def test_validate_valid_config(self):
        """Test validation of valid config."""
        config = KurigramConfig(api_url="https://api.kurigram.com", api_key="valid_key")
        config.validate()  # Should not raise

    def test_validate_missing_api_key(self):
        """Test validation with missing API key."""
        config = KurigramConfig(api_url="https://api.kurigram.com")

        with pytest.raises(ValueError, match="API key is required"):
            config.validate()

    def test_validate_invalid_url(self):
        """Test validation with invalid URL."""
        config = KurigramConfig(api_url="not-a-url", api_key="test_key")

        with pytest.raises(ValueError, match="Invalid API URL"):
            config.validate()


class TestKurigramClient:
    """Test KurigramClient class."""

    @pytest.fixture
    def config(self):
        """Create test config."""
        return KurigramConfig(api_url="https://test.kurigram.com", api_key="test_api_key")

    @pytest.fixture
    def client(self, config):
        """Create test client."""
        return KurigramClient(config)

    @pytest.fixture
    def mock_httpx_client(self):
        """Mock httpx client."""
        with patch("integrations.kurigram_client.httpx.AsyncClient") as mock_class:
            client = AsyncMock()
            mock_class.return_value = client
            client.__aenter__.return_value = client
            client.__aexit__.return_value = None
            yield client

    @pytest.mark.asyncio
    async def test_create_graph(self, client, mock_httpx_client):
        """Test creating a graph."""
        mock_response = MagicMock()
        mock_response.status_code = 201
        mock_response.json.return_value = {
            "id": "graph123",
            "name": "Test Graph",
            "description": "Test description",
            "entity_count": 0,
            "relationship_count": 0,
        }
        mock_httpx_client.post.return_value = mock_response

        graph = await client.create_graph(name="Test Graph", description="Test description")

        assert isinstance(graph, GraphData)
        assert graph.id == "graph123"
        assert graph.name == "Test Graph"

        mock_httpx_client.post.assert_called_once_with(
            "https://test.kurigram.com/graphs",
            json={"name": "Test Graph", "description": "Test description"},
            headers={"Authorization": "Bearer test_api_key"},
        )

    @pytest.mark.asyncio
    async def test_get_graph(self, client, mock_httpx_client):
        """Test getting a graph."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "id": "graph123",
            "name": "Test Graph",
            "entity_count": 10,
            "relationship_count": 15,
        }
        mock_httpx_client.get.return_value = mock_response

        graph = await client.get_graph("graph123")

        assert graph.id == "graph123"
        assert graph.entity_count == 10
        assert graph.relationship_count == 15

    @pytest.mark.asyncio
    async def test_add_entity(self, client, mock_httpx_client):
        """Test adding an entity."""
        mock_response = MagicMock()
        mock_response.status_code = 201
        mock_response.json.return_value = {
            "id": "entity123",
            "type": "Person",
            "properties": {"name": "John", "age": 30},
        }
        mock_httpx_client.post.return_value = mock_response

        entity = await client.add_entity(
            graph_id="graph123", entity_type="Person", properties={"name": "John", "age": 30}
        )

        assert isinstance(entity, Entity)
        assert entity.id == "entity123"
        assert entity.type == "Person"
        assert entity.properties["name"] == "John"

    @pytest.mark.asyncio
    async def test_add_entities_batch(self, client, mock_httpx_client):
        """Test adding multiple entities."""
        mock_response = MagicMock()
        mock_response.status_code = 201
        mock_response.json.return_value = {
            "entities": [
                {"id": "e1", "type": "Person", "properties": {"name": "Alice"}},
                {"id": "e2", "type": "Person", "properties": {"name": "Bob"}},
            ]
        }
        mock_httpx_client.post.return_value = mock_response

        entities = await client.add_entities(
            graph_id="graph123",
            entities=[
                {"type": "Person", "properties": {"name": "Alice"}},
                {"type": "Person", "properties": {"name": "Bob"}},
            ],
        )

        assert len(entities) == 2
        assert entities[0].properties["name"] == "Alice"
        assert entities[1].properties["name"] == "Bob"

    @pytest.mark.asyncio
    async def test_add_relationship(self, client, mock_httpx_client):
        """Test adding a relationship."""
        mock_response = MagicMock()
        mock_response.status_code = 201
        mock_response.json.return_value = {
            "id": "rel123",
            "type": "KNOWS",
            "source_id": "e1",
            "target_id": "e2",
            "properties": {"since": "2020"},
        }
        mock_httpx_client.post.return_value = mock_response

        relationship = await client.add_relationship(
            graph_id="graph123", source_id="e1", target_id="e2", relationship_type="KNOWS", properties={"since": "2020"}
        )

        assert isinstance(relationship, Relationship)
        assert relationship.type == "KNOWS"
        assert relationship.properties["since"] == "2020"

    @pytest.mark.asyncio
    async def test_query_graph(self, client, mock_httpx_client):
        """Test querying a graph."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "results": [
                {
                    "entities": [{"id": "e1", "type": "Person", "properties": {"name": "John"}}],
                    "relationships": [{"id": "r1", "type": "KNOWS", "source_id": "e1", "target_id": "e2"}],
                }
            ],
            "total": 1,
        }
        mock_httpx_client.post.return_value = mock_response

        query = Query(match_pattern="(p:Person {name: 'John'})", return_fields=["p"])

        result = await client.query(graph_id="graph123", query=query)

        assert isinstance(result, QueryResult)
        assert result.total == 1
        assert len(result.results) == 1
        assert len(result.results[0]["entities"]) == 1

    @pytest.mark.asyncio
    async def test_find_paths(self, client, mock_httpx_client):
        """Test finding paths between entities."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "paths": [{"entities": ["e1", "e2", "e3"], "relationships": ["r1", "r2"], "length": 2}]
        }
        mock_httpx_client.post.return_value = mock_response

        paths = await client.find_paths(graph_id="graph123", start_id="e1", end_id="e3", max_depth=3)

        assert len(paths) == 1
        assert paths[0]["length"] == 2
        assert len(paths[0]["entities"]) == 3

    @pytest.mark.asyncio
    async def test_get_entity_neighbors(self, client, mock_httpx_client):
        """Test getting entity neighbors."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "neighbors": [
                {"id": "e2", "type": "Person", "properties": {"name": "Alice"}},
                {"id": "e3", "type": "Person", "properties": {"name": "Bob"}},
            ],
            "relationships": [{"id": "r1", "type": "KNOWS"}, {"id": "r2", "type": "WORKS_WITH"}],
        }
        mock_httpx_client.get.return_value = mock_response

        result = await client.get_neighbors(
            graph_id="graph123", entity_id="e1", relationship_types=["KNOWS", "WORKS_WITH"]
        )

        assert len(result["neighbors"]) == 2
        assert len(result["relationships"]) == 2

    @pytest.mark.asyncio
    async def test_delete_entity(self, client, mock_httpx_client):
        """Test deleting an entity."""
        mock_response = MagicMock()
        mock_response.status_code = 204
        mock_httpx_client.delete.return_value = mock_response

        await client.delete_entity("graph123", "entity123")

        mock_httpx_client.delete.assert_called_once_with(
            "https://test.kurigram.com/graphs/graph123/entities/entity123",
            headers={"Authorization": "Bearer test_api_key"},
        )

    @pytest.mark.asyncio
    async def test_delete_graph(self, client, mock_httpx_client):
        """Test deleting a graph."""
        mock_response = MagicMock()
        mock_response.status_code = 204
        mock_httpx_client.delete.return_value = mock_response

        await client.delete_graph("graph123")

        mock_httpx_client.delete.assert_called_once()

    @pytest.mark.asyncio
    async def test_export_graph(self, client, mock_httpx_client):
        """Test exporting a graph."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "format": "json",
            "data": {"entities": [{"id": "e1", "type": "Person"}], "relationships": [{"id": "r1", "type": "KNOWS"}]},
        }
        mock_httpx_client.get.return_value = mock_response

        export_data = await client.export_graph(graph_id="graph123", format="json")

        assert export_data["format"] == "json"
        assert "entities" in export_data["data"]

    @pytest.mark.asyncio
    async def test_import_graph(self, client, mock_httpx_client):
        """Test importing graph data."""
        mock_response = MagicMock()
        mock_response.status_code = 201
        mock_response.json.return_value = {"graph_id": "new_graph", "imported": {"entities": 10, "relationships": 15}}
        mock_httpx_client.post.return_value = mock_response

        import_data = {"entities": [{"type": "Person", "properties": {"name": "Test"}}], "relationships": []}

        result = await client.import_graph(name="Imported Graph", data=import_data)

        assert result["graph_id"] == "new_graph"
        assert result["imported"]["entities"] == 10

    @pytest.mark.asyncio
    async def test_error_handling(self, client, mock_httpx_client):
        """Test error handling."""
        mock_response = MagicMock()
        mock_response.status_code = 400
        mock_response.json.return_value = {"error": "Invalid request"}
        mock_httpx_client.post.return_value = mock_response

        with pytest.raises(KurigramError, match="Invalid request"):
            await client.create_graph("Test")

    @pytest.mark.asyncio
    async def test_retry_on_timeout(self, client, mock_httpx_client):
        """Test retry logic on timeout."""
        import httpx

        # First call times out, second succeeds
        mock_httpx_client.get.side_effect = [
            httpx.TimeoutException("Timeout"),
            MagicMock(status_code=200, json=MagicMock(return_value={"id": "test"})),
        ]

        result = await client.get_graph("test")

        assert result["id"] == "test"
        assert mock_httpx_client.get.call_count == 2


class TestDataClasses:
    """Test data classes."""

    def test_entity_creation(self):
        """Test Entity creation."""
        entity = Entity(id="e1", type="Person", properties={"name": "John", "age": 30})

        assert entity.id == "e1"
        assert entity.type == "Person"
        assert entity.properties["name"] == "John"

    def test_relationship_creation(self):
        """Test Relationship creation."""
        rel = Relationship(id="r1", type="KNOWS", source_id="e1", target_id="e2", properties={"since": "2020"})

        assert rel.type == "KNOWS"
        assert rel.source_id == "e1"
        assert rel.properties["since"] == "2020"

    def test_query_creation(self):
        """Test Query creation."""
        query = Query(
            match_pattern="(p:Person)", where_clause="p.age > 25", return_fields=["p.name", "p.age"], limit=10
        )

        assert query.match_pattern == "(p:Person)"
        assert query.limit == 10

    def test_graph_data_creation(self):
        """Test GraphData creation."""
        graph = GraphData(id="g1", name="Test Graph", description="Test", entity_count=100, relationship_count=200)

        assert graph.name == "Test Graph"
        assert graph.entity_count == 100
