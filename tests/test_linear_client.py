"""Tests for Linear API client."""

from datetime import datetime, timezone
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from linear_client import (
    Comment,
    Issue,
    LinearClient,
    LinearConfig,
    LinearError,
    Team,
    User,
    add_comment,
    create_client,
    create_issue,
    get_issue,
    list_issues,
    update_issue,
)


class TestLinearConfig:
    """Test LinearConfig class."""

    def test_from_env(self, monkeypatch):
        """Test loading config from environment."""
        monkeypatch.setenv("LINEAR_API_KEY", "test_api_key")
        monkeypatch.setenv("LINEAR_API_URL", "https://api.linear.app/custom")

        config = LinearConfig.from_env()

        assert config.api_key == "test_api_key"
        assert config.api_url == "https://api.linear.app/custom"

    def test_from_env_defaults(self, monkeypatch):
        """Test loading config with defaults."""
        monkeypatch.setenv("LINEAR_API_KEY", "test_key")
        monkeypatch.delenv("LINEAR_API_URL", raising=False)

        config = LinearConfig.from_env()

        assert config.api_url == "https://api.linear.app/graphql"

    def test_from_env_missing_key(self, monkeypatch):
        """Test error when API key is missing."""
        monkeypatch.delenv("LINEAR_API_KEY", raising=False)

        with pytest.raises(ValueError, match="LINEAR_API_KEY"):
            LinearConfig.from_env()


class TestLinearClient:
    """Test LinearClient class."""

    @pytest.fixture
    def config(self):
        """Create test config."""
        return LinearConfig(api_key="test_api_key")

    @pytest.fixture
    def client(self, config):
        """Create test client."""
        return LinearClient(config)

    @pytest.fixture
    def mock_httpx_client(self):
        """Mock httpx client."""
        with patch("linear_client.httpx.AsyncClient") as mock_class:
            client = AsyncMock()
            mock_class.return_value = client
            client.__aenter__.return_value = client
            client.__aexit__.return_value = None
            yield client

    @pytest.mark.asyncio
    async def test_query(self, client, mock_httpx_client):
        """Test GraphQL query execution."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"data": {"viewer": {"id": "user123", "name": "Test User"}}}
        mock_httpx_client.post.return_value = mock_response

        query = """
        query {
            viewer {
                id
                name
            }
        }
        """

        result = await client.query(query)

        assert result["viewer"]["id"] == "user123"
        mock_httpx_client.post.assert_called_once_with(
            client.config.api_url, json={"query": query, "variables": {}}, headers={"Authorization": "test_api_key"}
        )

    @pytest.mark.asyncio
    async def test_query_with_variables(self, client, mock_httpx_client):
        """Test GraphQL query with variables."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"data": {"issue": {"id": "123"}}}
        mock_httpx_client.post.return_value = mock_response

        query = "query($id: String!) { issue(id: $id) { id } }"
        variables = {"id": "123"}

        result = await client.query(query, variables)

        assert result["issue"]["id"] == "123"

        call_json = mock_httpx_client.post.call_args[1]["json"]
        assert call_json["variables"] == variables

    @pytest.mark.asyncio
    async def test_query_error(self, client, mock_httpx_client):
        """Test handling GraphQL errors."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"errors": [{"message": "Field not found"}]}
        mock_httpx_client.post.return_value = mock_response

        with pytest.raises(LinearError, match="Field not found"):
            await client.query("{ invalid }")

    @pytest.mark.asyncio
    async def test_create_issue(self, client, mock_httpx_client):
        """Test creating an issue."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "issueCreate": {
                    "success": True,
                    "issue": {
                        "id": "issue123",
                        "title": "Test Issue",
                        "description": "Test description",
                        "identifier": "TEST-1",
                        "url": "https://linear.app/test/issue/TEST-1",
                        "createdAt": "2025-01-01T00:00:00Z",
                    },
                }
            }
        }
        mock_httpx_client.post.return_value = mock_response

        issue = await client.create_issue(
            title="Test Issue",
            team_id="team123",
            description="Test description",
            assignee_id="user123",
            priority=2,
            labels=["bug", "urgent"],
        )

        assert isinstance(issue, Issue)
        assert issue.id == "issue123"
        assert issue.title == "Test Issue"
        assert issue.identifier == "TEST-1"

    @pytest.mark.asyncio
    async def test_get_issue(self, client, mock_httpx_client):
        """Test getting an issue."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "issue": {
                    "id": "issue123",
                    "title": "Test Issue",
                    "state": {"name": "In Progress"},
                    "assignee": {"name": "John Doe"},
                }
            }
        }
        mock_httpx_client.post.return_value = mock_response

        issue = await client.get_issue("issue123")

        assert issue["title"] == "Test Issue"
        assert issue["state"]["name"] == "In Progress"

    @pytest.mark.asyncio
    async def test_update_issue(self, client, mock_httpx_client):
        """Test updating an issue."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {"issueUpdate": {"success": True, "issue": {"id": "issue123", "title": "Updated Title"}}}
        }
        mock_httpx_client.post.return_value = mock_response

        result = await client.update_issue(issue_id="issue123", title="Updated Title", state_id="done")

        assert result["success"] is True
        assert result["issue"]["title"] == "Updated Title"

    @pytest.mark.asyncio
    async def test_list_issues(self, client, mock_httpx_client):
        """Test listing issues."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "issues": {
                    "nodes": [{"id": "1", "title": "Issue 1"}, {"id": "2", "title": "Issue 2"}],
                    "pageInfo": {"hasNextPage": False, "endCursor": None},
                }
            }
        }
        mock_httpx_client.post.return_value = mock_response

        issues = await client.list_issues(team_id="team123", state_name="In Progress", assignee_id="user123")

        assert len(issues) == 2
        assert issues[0]["title"] == "Issue 1"

    @pytest.mark.asyncio
    async def test_add_comment(self, client, mock_httpx_client):
        """Test adding a comment to an issue."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "commentCreate": {
                    "success": True,
                    "comment": {"id": "comment123", "body": "Test comment", "createdAt": "2025-01-01T00:00:00Z"},
                }
            }
        }
        mock_httpx_client.post.return_value = mock_response

        comment = await client.add_comment(issue_id="issue123", body="Test comment")

        assert isinstance(comment, Comment)
        assert comment.id == "comment123"
        assert comment.body == "Test comment"

    @pytest.mark.asyncio
    async def test_get_teams(self, client, mock_httpx_client):
        """Test getting teams."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "teams": {
                    "nodes": [
                        {"id": "team1", "name": "Engineering", "key": "ENG"},
                        {"id": "team2", "name": "Design", "key": "DES"},
                    ]
                }
            }
        }
        mock_httpx_client.post.return_value = mock_response

        teams = await client.get_teams()

        assert len(teams) == 2
        assert teams[0]["name"] == "Engineering"

    @pytest.mark.asyncio
    async def test_get_users(self, client, mock_httpx_client):
        """Test getting users."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "users": {
                    "nodes": [
                        {"id": "user1", "name": "John Doe", "email": "john@example.com"},
                        {"id": "user2", "name": "Jane Smith", "email": "jane@example.com"},
                    ]
                }
            }
        }
        mock_httpx_client.post.return_value = mock_response

        users = await client.get_users()

        assert len(users) == 2
        assert users[0]["email"] == "john@example.com"

    @pytest.mark.asyncio
    async def test_get_labels(self, client, mock_httpx_client):
        """Test getting labels."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "issueLabels": {
                    "nodes": [
                        {"id": "label1", "name": "bug", "color": "#FF0000"},
                        {"id": "label2", "name": "feature", "color": "#00FF00"},
                    ]
                }
            }
        }
        mock_httpx_client.post.return_value = mock_response

        labels = await client.get_labels()

        assert len(labels) == 2
        assert labels[0]["name"] == "bug"
        assert labels[1]["color"] == "#00FF00"

    @pytest.mark.asyncio
    async def test_search_issues(self, client, mock_httpx_client):
        """Test searching issues."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": {
                "issueSearch": {"nodes": [{"id": "1", "title": "Bug in login"}, {"id": "2", "title": "Bug in signup"}]}
            }
        }
        mock_httpx_client.post.return_value = mock_response

        results = await client.search_issues("bug")

        assert len(results) == 2
        assert "login" in results[0]["title"]

    @pytest.mark.asyncio
    async def test_http_error(self, client, mock_httpx_client):
        """Test handling HTTP errors."""
        mock_response = MagicMock()
        mock_response.status_code = 401
        mock_response.json.return_value = {"error": "Unauthorized"}
        mock_httpx_client.post.return_value = mock_response

        with pytest.raises(LinearError, match="401"):
            await client.query("{ viewer { id } }")


class TestDataClasses:
    """Test data classes."""

    def test_issue_creation(self):
        """Test Issue dataclass."""
        issue = Issue(
            id="123",
            title="Test Issue",
            description="Description",
            identifier="TEST-1",
            url="https://linear.app/test/issue/TEST-1",
            created_at=datetime.now(timezone.utc),
        )

        assert issue.id == "123"
        assert issue.identifier == "TEST-1"

    def test_team_creation(self):
        """Test Team dataclass."""
        team = Team(id="team123", name="Engineering", key="ENG", description="Engineering team")

        assert team.name == "Engineering"
        assert team.key == "ENG"

    def test_user_creation(self):
        """Test User dataclass."""
        user = User(
            id="user123", name="John Doe", email="john@example.com", avatar_url="https://example.com/avatar.jpg"
        )

        assert user.name == "John Doe"
        assert user.email == "john@example.com"


class TestModuleFunctions:
    """Test module-level convenience functions."""

    @patch("linear_client.LinearClient")
    def test_create_client_function(self, mock_client_class, mock_env):
        """Test create_client function."""
        create_client()

        mock_client_class.assert_called_once()

    @patch("linear_client.LinearClient")
    @pytest.mark.asyncio
    async def test_create_issue_function(self, mock_client_class):
        """Test create_issue convenience function."""
        mock_client = AsyncMock()
        mock_client_class.return_value = mock_client
        mock_client.create_issue.return_value = Issue(
            id="123", title="Test", identifier="TEST-1", url="https://linear.app", created_at=datetime.now(timezone.utc)
        )

        issue = await create_issue(title="Test", team_id="team123", api_key="test_key")

        assert issue.id == "123"

    @patch("linear_client.LinearClient")
    @pytest.mark.asyncio
    async def test_get_issue_function(self, mock_client_class):
        """Test get_issue convenience function."""
        mock_client = AsyncMock()
        mock_client_class.return_value = mock_client
        mock_client.get_issue.return_value = {"id": "123", "title": "Test"}

        issue = await get_issue("123", api_key="test_key")

        assert issue["title"] == "Test"

    @patch("linear_client.LinearClient")
    @pytest.mark.asyncio
    async def test_update_issue_function(self, mock_client_class):
        """Test update_issue convenience function."""
        mock_client = AsyncMock()
        mock_client_class.return_value = mock_client
        mock_client.update_issue.return_value = {"success": True}

        result = await update_issue(issue_id="123", title="Updated", api_key="test_key")

        assert result["success"] is True

    @patch("linear_client.LinearClient")
    @pytest.mark.asyncio
    async def test_list_issues_function(self, mock_client_class):
        """Test list_issues convenience function."""
        mock_client = AsyncMock()
        mock_client_class.return_value = mock_client
        mock_client.list_issues.return_value = [{"id": "1", "title": "Issue 1"}, {"id": "2", "title": "Issue 2"}]

        issues = await list_issues(api_key="test_key")

        assert len(issues) == 2

    @patch("linear_client.LinearClient")
    @pytest.mark.asyncio
    async def test_add_comment_function(self, mock_client_class):
        """Test add_comment convenience function."""
        mock_client = AsyncMock()
        mock_client_class.return_value = mock_client
        mock_client.add_comment.return_value = Comment(
            id="comment123", body="Test comment", created_at=datetime.now(timezone.utc)
        )

        comment = await add_comment(issue_id="123", body="Test comment", api_key="test_key")

        assert comment.body == "Test comment"
