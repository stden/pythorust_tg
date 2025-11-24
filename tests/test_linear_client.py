# -*- coding: utf-8 -*-
"""Tests for Linear API client."""
import os
import pytest
from unittest.mock import MagicMock, patch

# Set LINEAR_API_KEY before imports
os.environ.setdefault("LINEAR_API_KEY", "test-api-key")


class TestLinearError:
    """Tests for LinearError exception."""

    def test_linear_error(self):
        from linear_client import LinearError

        with pytest.raises(LinearError):
            raise LinearError("Test error")


class TestLinearClient:
    """Tests for LinearClient class."""

    def test_init_without_api_key_raises(self):
        from linear_client import LinearClient, LinearError

        # Use None to bypass the explicit api_key param and force env lookup
        with patch.dict(os.environ, {"LINEAR_API_KEY": ""}, clear=False):
            with pytest.raises(LinearError, match="LINEAR_API_KEY"):
                LinearClient(api_key=None)

    def test_init_with_api_key(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-api-key")
        assert client.api_key == "test-api-key"
        assert client.timeout == 10

    def test_init_custom_timeout(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-key", timeout=30)
        assert client.timeout == 30

    def test_request_success(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-key")

        with patch.object(client.session, "post") as mock_post:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.json.return_value = {"data": {"teams": []}}
            mock_post.return_value = mock_response

            result = client._request("query { teams { id } }", {})
            assert result == {"teams": []}

    def test_request_http_error(self):
        from linear_client import LinearClient, LinearError

        client = LinearClient(api_key="test-key")

        with patch.object(client.session, "post") as mock_post:
            mock_response = MagicMock()
            mock_response.status_code = 500
            mock_response.text = "Internal Server Error"
            mock_post.return_value = mock_response

            with pytest.raises(LinearError, match="статусом 500"):
                client._request("query { teams }", {})

    def test_request_api_error(self):
        from linear_client import LinearClient, LinearError

        client = LinearClient(api_key="test-key")

        with patch.object(client.session, "post") as mock_post:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.json.return_value = {
                "errors": [{"message": "Not authorized"}]
            }
            mock_post.return_value = mock_response

            with pytest.raises(LinearError, match="Not authorized"):
                client._request("query { teams }", {})

    def test_get_team_id_cached(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-key")
        client._team_cache["TEST"] = "team-id-123"

        result = client.get_team_id("TEST")
        assert result == "team-id-123"

    def test_get_team_id_from_api(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-key")

        with patch.object(client, "_request") as mock_request:
            mock_request.return_value = {
                "teams": {
                    "nodes": [{"id": "team-123", "name": "Test Team", "key": "TEST"}]
                }
            }

            result = client.get_team_id("TEST")
            assert result == "team-123"
            assert client._team_cache["TEST"] == "team-123"

    def test_get_team_id_not_found(self):
        from linear_client import LinearClient, LinearError

        client = LinearClient(api_key="test-key")

        with patch.object(client, "_request") as mock_request:
            mock_request.return_value = {"teams": {"nodes": []}}

            with pytest.raises(LinearError, match="не найдена"):
                client.get_team_id("NONEXISTENT")

    def test_create_issue_success(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-key")

        with patch.object(client, "get_team_id") as mock_team:
            mock_team.return_value = "team-123"

            with patch.object(client, "_request") as mock_request:
                mock_request.return_value = {
                    "issueCreate": {
                        "success": True,
                        "issue": {
                            "id": "issue-123",
                            "identifier": "TEST-1",
                            "title": "Test Issue",
                            "url": "https://linear.app/test/issue/TEST-1"
                        }
                    }
                }

                result = client.create_issue(team_key="TEST", title="Test Issue")
                assert result["id"] == "issue-123"

    def test_create_issue_empty_title(self):
        from linear_client import LinearClient, LinearError

        client = LinearClient(api_key="test-key")

        with pytest.raises(LinearError, match="Заголовок задачи пустой"):
            client.create_issue(team_key="TEST", title="")

    def test_create_issue_failed(self):
        from linear_client import LinearClient, LinearError

        client = LinearClient(api_key="test-key")

        with patch.object(client, "get_team_id") as mock_team:
            mock_team.return_value = "team-123"

            with patch.object(client, "_request") as mock_request:
                mock_request.return_value = {"issueCreate": {"success": False}}

                with pytest.raises(LinearError, match="не подтвердил создание"):
                    client.create_issue(team_key="TEST", title="Test")

    def test_update_issue_success(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-key")

        with patch.object(client, "_request") as mock_request:
            mock_request.return_value = {
                "issueUpdate": {
                    "success": True,
                    "issue": {
                        "id": "issue-123",
                        "identifier": "TEST-1",
                        "title": "Updated",
                        "dueDate": "2024-12-31",
                        "url": "https://linear.app/test/issue/TEST-1"
                    }
                }
            }

            result = client.update_issue(issue_id="issue-123", due_date="2024-12-31")
            assert result["dueDate"] == "2024-12-31"

    def test_update_issue_no_data(self):
        from linear_client import LinearClient, LinearError

        client = LinearClient(api_key="test-key")

        with pytest.raises(LinearError, match="Нет данных для обновления"):
            client.update_issue(issue_id="issue-123")

    def test_find_issue_by_identifier_found(self):
        from linear_client import LinearClient

        client = LinearClient(api_key="test-key")

        with patch.object(client, "_request") as mock_request:
            mock_request.return_value = {
                "issue": {
                    "id": "issue-123",
                    "identifier": "TEST-1",
                    "title": "Test Issue",
                    "url": "https://linear.app/test/issue/TEST-1"
                }
            }

            result = client.find_issue_by_identifier("TEST-1")
            assert result["id"] == "issue-123"

    def test_find_issue_by_identifier_not_found(self):
        from linear_client import LinearClient, LinearError

        client = LinearClient(api_key="test-key")

        with patch.object(client, "_request") as mock_request:
            mock_request.side_effect = LinearError("Not found")

            result = client.find_issue_by_identifier("NONEXISTENT-999")
            assert result is None
