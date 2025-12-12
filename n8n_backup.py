#!/usr/bin/env python3
"""
N8N Configuration Backup Script
Автоматический бэкап конфигураций N8N
"""

import asyncio
import aiohttp
import json
import logging
from datetime import datetime
from pathlib import Path
import tarfile
import os
import sys
from dotenv import load_dotenv

load_dotenv()

# Configuration
N8N_URL = os.getenv("N8N_URL")
N8N_API_KEY = os.getenv("N8N_API_KEY")
BACKUP_DIR = Path(os.getenv("BACKUP_DIR"))
RETENTION_DAYS = int(os.getenv("RETENTION_DAYS"))
MAX_BACKUPS = int(os.getenv("MAX_BACKUPS"))

logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s")
logger = logging.getLogger(__name__)


class N8NBackup:
    """N8N backup manager."""

    def __init__(self):
        self.backup_dir = BACKUP_DIR
        self.backup_dir.mkdir(parents=True, exist_ok=True)

    async def get_workflows(self) -> list:
        """Get all workflows from N8N API."""
        try:
            async with aiohttp.ClientSession() as session:
                headers = {}
                if N8N_API_KEY:
                    headers["X-N8N-API-KEY"] = N8N_API_KEY

                async with session.get(f"{N8N_URL}/api/v1/workflows", headers=headers, ssl=False) as response:
                    if response.status == 200:
                        data = await response.json()
                        workflows = data.get("data", [])
                        logger.info(f"✅ Retrieved {len(workflows)} workflows")
                        return workflows
                    else:
                        logger.error(f"❌ Failed to get workflows: HTTP {response.status}")
                        return []
        except Exception as e:
            logger.error(f"❌ Error getting workflows: {e}")
            return []

    async def get_credentials(self) -> list:
        """Get all credentials from N8N API (if accessible)."""
        try:
            async with aiohttp.ClientSession() as session:
                headers = {}
                if N8N_API_KEY:
                    headers["X-N8N-API-KEY"] = N8N_API_KEY

                async with session.get(f"{N8N_URL}/api/v1/credentials", headers=headers, ssl=False) as response:
                    if response.status == 200:
                        data = await response.json()
                        credentials = data.get("data", [])
                        logger.info(f"✅ Retrieved {len(credentials)} credentials (metadata only)")
                        return credentials
                    else:
                        logger.warning(f"⚠️ Could not get credentials: HTTP {response.status}")
                        return []
        except Exception as e:
            logger.warning(f"⚠️ Could not get credentials: {e}")
            return []

    async def create_backup(self) -> Path:
        """Create a backup of N8N configuration."""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_name = f"n8n_backup_{timestamp}"
        backup_path = self.backup_dir / backup_name
        backup_path.mkdir(parents=True, exist_ok=True)

        logger.info(f"🔄 Creating backup: {backup_name}")

        # 1. Backup workflows
        workflows = await self.get_workflows()
        if workflows:
            workflows_file = backup_path / "workflows.json"
            with open(workflows_file, "w", encoding="utf-8") as f:
                json.dump(workflows, f, indent=2, ensure_ascii=False)
            logger.info(f"✅ Saved {len(workflows)} workflows")

        # 2. Backup credentials metadata (without sensitive data)
        credentials = await self.get_credentials()
        if credentials:
            credentials_file = backup_path / "credentials_meta.json"
            with open(credentials_file, "w", encoding="utf-8") as f:
                json.dump(credentials, f, indent=2, ensure_ascii=False)
            logger.info(f"✅ Saved {len(credentials)} credentials metadata")

        # 3. Create backup info file
        backup_info = {
            "timestamp": timestamp,
            "datetime": datetime.now().isoformat(),
            "n8n_url": N8N_URL,
            "workflows_count": len(workflows),
            "credentials_count": len(credentials),
        }
        info_file = backup_path / "backup_info.json"
        with open(info_file, "w", encoding="utf-8") as f:
            json.dump(backup_info, f, indent=2)

        # 4. Create tar.gz archive
        archive_path = self.backup_dir / f"{backup_name}.tar.gz"
        with tarfile.open(archive_path, "w:gz") as tar:
            tar.add(backup_path, arcname=backup_name)

        logger.info(f"✅ Created archive: {archive_path}")

        # 5. Cleanup temporary directory
        import shutil

        shutil.rmtree(backup_path)

        return archive_path

    async def cleanup_old_backups(self):
        """Remove old backups based on retention policy."""
        backups = sorted(self.backup_dir.glob("n8n_backup_*.tar.gz"))

        # Remove by age
        now = datetime.now()
        removed_by_age = 0
        for backup in backups:
            mtime = datetime.fromtimestamp(backup.stat().st_mtime)
            age_days = (now - mtime).days
            if age_days > RETENTION_DAYS:
                backup.unlink()
                removed_by_age += 1
                logger.info(f"🗑️ Removed old backup: {backup.name} (age: {age_days} days)")

        # Remove by count
        backups = sorted(self.backup_dir.glob("n8n_backup_*.tar.gz"), key=lambda p: p.stat().st_mtime)
        removed_by_count = 0
        while len(backups) > MAX_BACKUPS:
            oldest = backups.pop(0)
            oldest.unlink()
            removed_by_count += 1
            logger.info(f"🗑️ Removed excess backup: {oldest.name}")

        if removed_by_age or removed_by_count:
            logger.info(f"✅ Cleanup: removed {removed_by_age} by age, {removed_by_count} by count")
        else:
            logger.info("✅ No backups to remove")

    async def restore_backup(self, backup_file: Path):
        """Restore N8N configuration from backup."""
        if not backup_file.exists():
            logger.error(f"❌ Backup file not found: {backup_file}")
            return False

        logger.info(f"🔄 Restoring from: {backup_file}")

        # Extract archive
        temp_dir = self.backup_dir / "restore_temp"
        temp_dir.mkdir(parents=True, exist_ok=True)

        try:
            with tarfile.open(backup_file, "r:gz") as tar:
                tar.extractall(temp_dir)

            # Find extracted directory
            extracted_dirs = list(temp_dir.glob("n8n_backup_*"))
            if not extracted_dirs:
                logger.error("❌ No backup data found in archive")
                return False

            backup_data_dir = extracted_dirs[0]

            # Restore workflows
            workflows_file = backup_data_dir / "workflows.json"
            if workflows_file.exists():
                with open(workflows_file, "r", encoding="utf-8") as f:
                    workflows = json.load(f)

                logger.info(f"🔄 Restoring {len(workflows)} workflows...")
                # TODO: Implement workflow restoration via API
                logger.warning("⚠️ Workflow restoration via API not yet implemented")
                logger.info("ℹ️ You can manually import workflows from:")
                logger.info(f"   {workflows_file}")

            logger.info("✅ Backup extracted successfully")
            return True

        except Exception as e:
            logger.error(f"❌ Error restoring backup: {e}")
            return False

        finally:
            # Cleanup temp directory
            import shutil

            if temp_dir.exists():
                shutil.rmtree(temp_dir)

    async def list_backups(self):
        """List all available backups."""
        backups = sorted(self.backup_dir.glob("n8n_backup_*.tar.gz"), key=lambda p: p.stat().st_mtime, reverse=True)

        if not backups:
            logger.info("📦 No backups found")
            return

        logger.info(f"📦 Available backups ({len(backups)}):")
        for backup in backups:
            stat = backup.stat()
            size_mb = stat.st_size / (1024 * 1024)
            mtime = datetime.fromtimestamp(stat.st_mtime)
            age_days = (datetime.now() - mtime).days
            logger.info(f"  • {backup.name} ({size_mb:.2f} MB, {age_days} days old)")


async def main():
    """Entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="N8N Backup Manager")
    parser.add_argument("action", choices=["backup", "restore", "list", "cleanup"], help="Action to perform")
    parser.add_argument("--file", type=Path, help="Backup file for restore action")

    args = parser.parse_args()

    backup_manager = N8NBackup()

    if args.action == "backup":
        archive = await backup_manager.create_backup()
        logger.info(f"✅ Backup created: {archive}")

    elif args.action == "restore":
        if not args.file:
            logger.error("❌ Please specify backup file with --file")
            sys.exit(1)
        await backup_manager.restore_backup(args.file)

    elif args.action == "list":
        await backup_manager.list_backups()

    elif args.action == "cleanup":
        await backup_manager.cleanup_old_backups()


if __name__ == "__main__":
    asyncio.run(main())
