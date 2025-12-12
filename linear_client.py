import os
from typing import Any, Dict, List, Optional

import requests
from dotenv import load_dotenv

load_dotenv()

LINEAR_API_URL = "https://api.linear.app/graphql"


class LinearError(Exception):
    """Ошибки при работе с Linear API."""


class LinearClient:
    """Минимальный клиент для Linear GraphQL API (создание задач)."""

    def __init__(self, api_key: Optional[str] = None, timeout: int = 10):
        self.api_key = api_key or os.getenv("LINEAR_API_KEY")
        if not self.api_key:
            raise LinearError("Не указан LINEAR_API_KEY (добавьте в .env или передайте явно).")

        self.timeout = timeout
        self._team_cache: Dict[str, str] = {}

        self.session = requests.Session()
        self.session.headers.update(
            {
                "Authorization": self.api_key,
                "Content-Type": "application/json",
            }
        )

    def _request(self, query: str, variables: Dict[str, Any]) -> Dict[str, Any]:
        try:
            response = self.session.post(
                LINEAR_API_URL,
                json={"query": query, "variables": variables},
                timeout=self.timeout,
            )
        except requests.RequestException as exc:
            raise LinearError(f"Не удалось обратиться к Linear: {exc}") from exc

        if response.status_code != 200:
            raise LinearError(f"Linear ответил статусом {response.status_code}: {response.text}")

        try:
            payload = response.json()
        except ValueError as exc:
            raise LinearError(f"Linear вернул не-JSON ответ: {response.text}") from exc

        if "errors" in payload:
            message = "; ".join(err.get("message", "Unknown error") for err in payload["errors"])
            raise LinearError(f"Linear API error: {message}")

        return payload.get("data", {})

    def get_team_id(self, team_key: str) -> str:
        if team_key in self._team_cache:
            return self._team_cache[team_key]

        # New API: query all teams and find by key
        query = """
        query Teams {
          teams {
            nodes {
              id
              name
              key
            }
          }
        }
        """
        data = self._request(query, {})
        teams = data.get("teams", {}).get("nodes", [])

        for team in teams:
            if team.get("key") == team_key:
                self._team_cache[team_key] = team["id"]
                return team["id"]

        raise LinearError(f"Команда '{team_key}' не найдена в Linear.")

    def create_issue(
        self,
        *,
        team_key: str,
        title: str,
        description: Optional[str] = None,
        project_id: Optional[str] = None,
        priority: Optional[int] = None,
        assignee_id: Optional[str] = None,
        label_ids: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        if not title:
            raise LinearError("Заголовок задачи пустой.")

        team_id = self.get_team_id(team_key)
        payload: Dict[str, Any] = {
            "teamId": team_id,
            "title": title.strip(),
        }

        if description:
            payload["description"] = description.strip()
        if project_id:
            payload["projectId"] = project_id
        if assignee_id:
            payload["assigneeId"] = assignee_id
        if label_ids:
            payload["labelIds"] = label_ids
        if priority is not None:
            payload["priority"] = max(0, min(4, int(priority)))

        mutation = """
        mutation IssueCreate($input: IssueCreateInput!) {
          issueCreate(input: $input) {
            success
            issue {
              id
              identifier
              title
              url
            }
          }
        }
        """

        data = self._request(mutation, {"input": payload})
        result = data.get("issueCreate") if data else None
        if not result or not result.get("success"):
            raise LinearError("Linear не подтвердил создание задачи.")

        return result["issue"]

    def update_issue(
        self,
        *,
        issue_id: str,
        due_date: Optional[str] = None,
        priority: Optional[int] = None,
        state_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Update an existing issue (e.g., set due date)."""
        payload: Dict[str, Any] = {}

        if due_date:
            payload["dueDate"] = due_date  # Format: YYYY-MM-DD
        if priority is not None:
            payload["priority"] = max(0, min(4, int(priority)))
        if state_id:
            payload["stateId"] = state_id

        if not payload:
            raise LinearError("Нет данных для обновления.")

        mutation = """
        mutation IssueUpdate($id: String!, $input: IssueUpdateInput!) {
          issueUpdate(id: $id, input: $input) {
            success
            issue {
              id
              identifier
              title
              dueDate
              url
            }
          }
        }
        """

        data = self._request(mutation, {"id": issue_id, "input": payload})
        result = data.get("issueUpdate") if data else None
        if not result or not result.get("success"):
            raise LinearError("Linear не подтвердил обновление задачи.")

        return result["issue"]

    def find_issue_by_identifier(self, identifier: str) -> Optional[Dict[str, Any]]:
        """Find issue by identifier like PER-389."""
        query = """
        query Issue($id: String!) {
          issue(id: $id) {
            id
            identifier
            title
            url
          }
        }
        """
        try:
            data = self._request(query, {"id": identifier})
            return data.get("issue")
        except LinearError:
            return None
