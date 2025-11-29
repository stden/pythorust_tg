#!/usr/bin/env python3
"""
Export Linear tasks to MySQL database.
"""

import os
import pymysql
import requests
from datetime import datetime
from typing import Dict, Any, List, Optional
from dotenv import load_dotenv

load_dotenv('/srv/pythorust_tg/.env')

LINEAR_API_URL = "https://api.linear.app/graphql"


def parse_iso_datetime(iso_str: Optional[str]) -> Optional[datetime]:
    """Convert ISO 8601 datetime string to Python datetime."""
    if not iso_str:
        return None
    try:
        # Remove 'Z' suffix and parse
        if iso_str.endswith('Z'):
            iso_str = iso_str[:-1] + '+00:00'
        return datetime.fromisoformat(iso_str.replace('Z', '+00:00'))
    except (ValueError, AttributeError):
        return None

# MySQL config
MYSQL_CONFIG = {
    'host': os.getenv('MYSQL_HOST', 'localhost'),
    'port': int(os.getenv('MYSQL_PORT', 3306)),
    'database': os.getenv('MYSQL_DATABASE', 'pythorust_tg'),
    'user': os.getenv('MYSQL_USER', 'pythorust_tg'),
    'password': os.getenv('MYSQL_PASSWORD'),
    'charset': 'utf8mb4',
    'cursorclass': pymysql.cursors.DictCursor
}


class LinearReader:
    """Read-only Linear API client."""

    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or os.getenv("LINEAR_API_KEY")
        if not self.api_key:
            raise ValueError("LINEAR_API_KEY not set")

        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": self.api_key,
            "Content-Type": "application/json",
        })

    def _request(self, query: str, variables: Dict[str, Any] = None) -> Dict[str, Any]:
        response = self.session.post(
            LINEAR_API_URL,
            json={"query": query, "variables": variables or {}},
            timeout=30,
        )

        if response.status_code != 200:
            raise Exception(f"Linear API error: {response.status_code}")

        payload = response.json()
        if "errors" in payload:
            raise Exception(f"Linear GraphQL error: {payload['errors']}")

        return payload.get("data", {})

    def get_all_issues(self, first: int = 250) -> List[Dict[str, Any]]:
        """Get all issues from Linear."""
        query = """
        query Issues($first: Int!, $after: String) {
          issues(first: $first, after: $after) {
            pageInfo {
              hasNextPage
              endCursor
            }
            nodes {
              id
              identifier
              title
              description
              priority
              priorityLabel
              state {
                id
                name
                type
              }
              team {
                id
                name
                key
              }
              project {
                id
                name
              }
              assignee {
                id
                name
                email
              }
              creator {
                id
                name
              }
              labels {
                nodes {
                  id
                  name
                  color
                }
              }
              dueDate
              estimate
              url
              createdAt
              updatedAt
              completedAt
              canceledAt
              startedAt
            }
          }
        }
        """

        all_issues = []
        after = None

        while True:
            data = self._request(query, {"first": first, "after": after})
            issues_data = data.get("issues", {})
            nodes = issues_data.get("nodes", [])
            all_issues.extend(nodes)

            page_info = issues_data.get("pageInfo", {})
            if not page_info.get("hasNextPage"):
                break
            after = page_info.get("endCursor")

        return all_issues

    def get_teams(self) -> List[Dict[str, Any]]:
        """Get all teams."""
        query = """
        query Teams {
          teams {
            nodes {
              id
              name
              key
              description
            }
          }
        }
        """
        data = self._request(query)
        return data.get("teams", {}).get("nodes", [])

    def get_projects(self, first: int = 100) -> List[Dict[str, Any]]:
        """Get all projects."""
        query = """
        query Projects($first: Int!) {
          projects(first: $first) {
            nodes {
              id
              name
              description
              state
              startDate
              targetDate
              progress
              teams {
                nodes {
                  id
                  name
                }
              }
            }
          }
        }
        """
        data = self._request(query, {"first": first})
        return data.get("projects", {}).get("nodes", [])


def create_tables(conn):
    """Create MySQL tables for Linear data."""
    cursor = conn.cursor()

    # Linear teams table
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS linear_teams (
            id VARCHAR(64) PRIMARY KEY,
            name VARCHAR(255),
            key_name VARCHAR(50),
            description TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        )
    """)

    # Linear projects table
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS linear_projects (
            id VARCHAR(64) PRIMARY KEY,
            name VARCHAR(255),
            description TEXT,
            state VARCHAR(50),
            start_date DATE,
            target_date DATE,
            progress FLOAT,
            team_id VARCHAR(64),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            INDEX idx_team_id (team_id)
        )
    """)

    # Linear issues (tasks) table
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS linear_issues (
            id VARCHAR(64) PRIMARY KEY,
            identifier VARCHAR(50) UNIQUE,
            title VARCHAR(500),
            description TEXT,
            priority INT,
            priority_label VARCHAR(50),
            state_id VARCHAR(64),
            state_name VARCHAR(100),
            state_type VARCHAR(50),
            team_id VARCHAR(64),
            team_name VARCHAR(255),
            project_id VARCHAR(64),
            project_name VARCHAR(255),
            assignee_id VARCHAR(64),
            assignee_name VARCHAR(255),
            creator_id VARCHAR(64),
            creator_name VARCHAR(255),
            labels JSON,
            due_date DATE,
            estimate INT,
            url VARCHAR(500),
            linear_created_at DATETIME,
            linear_updated_at DATETIME,
            completed_at DATETIME,
            canceled_at DATETIME,
            started_at DATETIME,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            INDEX idx_team_id (team_id),
            INDEX idx_project_id (project_id),
            INDEX idx_assignee_id (assignee_id),
            INDEX idx_state_type (state_type),
            INDEX idx_priority (priority),
            INDEX idx_due_date (due_date)
        )
    """)

    conn.commit()
    print("Tables created successfully")


def insert_team(conn, team: Dict[str, Any]):
    """Insert or update a team."""
    cursor = conn.cursor()
    cursor.execute("""
        INSERT INTO linear_teams (id, name, key_name, description)
        VALUES (%s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
            name = VALUES(name),
            key_name = VALUES(key_name),
            description = VALUES(description)
    """, (
        team['id'],
        team['name'],
        team.get('key'),
        team.get('description')
    ))
    conn.commit()


def insert_project(conn, project: Dict[str, Any]):
    """Insert or update a project."""
    cursor = conn.cursor()

    # Get first team from teams list
    teams = project.get('teams', {}).get('nodes', [])
    team = teams[0] if teams else {}

    cursor.execute("""
        INSERT INTO linear_projects (id, name, description, state, start_date, target_date, progress, team_id)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
            name = VALUES(name),
            description = VALUES(description),
            state = VALUES(state),
            start_date = VALUES(start_date),
            target_date = VALUES(target_date),
            progress = VALUES(progress),
            team_id = VALUES(team_id)
    """, (
        project['id'],
        project['name'],
        project.get('description'),
        project.get('state'),
        project.get('startDate'),
        project.get('targetDate'),
        project.get('progress'),
        team.get('id')
    ))
    conn.commit()


def insert_issue(conn, issue: Dict[str, Any]):
    """Insert or update an issue."""
    cursor = conn.cursor()

    state = issue.get('state') or {}
    team = issue.get('team') or {}
    project = issue.get('project') or {}
    assignee = issue.get('assignee') or {}
    creator = issue.get('creator') or {}
    labels = issue.get('labels', {}).get('nodes', [])

    import json
    labels_json = json.dumps([{
        'id': l['id'],
        'name': l['name'],
        'color': l.get('color')
    } for l in labels])

    cursor.execute("""
        INSERT INTO linear_issues (
            id, identifier, title, description, priority, priority_label,
            state_id, state_name, state_type,
            team_id, team_name, project_id, project_name,
            assignee_id, assignee_name, creator_id, creator_name,
            labels, due_date, estimate, url,
            linear_created_at, linear_updated_at, completed_at, canceled_at, started_at
        )
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
            title = VALUES(title),
            description = VALUES(description),
            priority = VALUES(priority),
            priority_label = VALUES(priority_label),
            state_id = VALUES(state_id),
            state_name = VALUES(state_name),
            state_type = VALUES(state_type),
            team_id = VALUES(team_id),
            team_name = VALUES(team_name),
            project_id = VALUES(project_id),
            project_name = VALUES(project_name),
            assignee_id = VALUES(assignee_id),
            assignee_name = VALUES(assignee_name),
            labels = VALUES(labels),
            due_date = VALUES(due_date),
            estimate = VALUES(estimate),
            linear_updated_at = VALUES(linear_updated_at),
            completed_at = VALUES(completed_at),
            canceled_at = VALUES(canceled_at),
            started_at = VALUES(started_at)
    """, (
        issue['id'],
        issue['identifier'],
        issue['title'],
        issue.get('description'),
        issue.get('priority'),
        issue.get('priorityLabel'),
        state.get('id'),
        state.get('name'),
        state.get('type'),
        team.get('id'),
        team.get('name'),
        project.get('id'),
        project.get('name'),
        assignee.get('id'),
        assignee.get('name'),
        creator.get('id'),
        creator.get('name'),
        labels_json,
        issue.get('dueDate'),
        issue.get('estimate'),
        issue.get('url'),
        parse_iso_datetime(issue.get('createdAt')),
        parse_iso_datetime(issue.get('updatedAt')),
        parse_iso_datetime(issue.get('completedAt')),
        parse_iso_datetime(issue.get('canceledAt')),
        parse_iso_datetime(issue.get('startedAt'))
    ))
    conn.commit()


def main():
    print("Connecting to Linear API...")
    linear = LinearReader()

    print("Connecting to MySQL...")
    conn = pymysql.connect(**MYSQL_CONFIG)

    print("Creating tables...")
    create_tables(conn)

    # Export teams
    print("\nExporting teams...")
    teams = linear.get_teams()
    for team in teams:
        insert_team(conn, team)
        print(f"  Team: {team['name']} ({team.get('key', 'N/A')})")
    print(f"Exported {len(teams)} teams")

    # Export projects
    print("\nExporting projects...")
    projects = linear.get_projects()
    for project in projects:
        insert_project(conn, project)
        print(f"  Project: {project['name']}")
    print(f"Exported {len(projects)} projects")

    # Export issues
    print("\nExporting issues...")
    issues = linear.get_all_issues()
    for issue in issues:
        insert_issue(conn, issue)
        state_name = issue.get('state', {}).get('name', 'Unknown')
        print(f"  [{state_name}] {issue['identifier']}: {issue['title'][:50]}...")
    print(f"\nExported {len(issues)} issues")

    # Summary
    cursor = conn.cursor()
    cursor.execute("SELECT state_type, COUNT(*) as cnt FROM linear_issues GROUP BY state_type")
    stats = cursor.fetchall()

    print("\n" + "=" * 50)
    print("Export Summary:")
    print(f"  Teams: {len(teams)}")
    print(f"  Projects: {len(projects)}")
    print(f"  Issues: {len(issues)}")
    print("\nIssues by state:")
    for stat in stats:
        print(f"  {stat['state_type'] or 'Unknown'}: {stat['cnt']}")

    conn.close()
    print("\nDone!")


if __name__ == '__main__':
    main()
