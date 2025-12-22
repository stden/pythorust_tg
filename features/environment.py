# -*- coding: utf-8 -*-
"""Behave test environment hooks."""

import os
import shutil
import sys
from pathlib import Path

# Add project root to path
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root))


def before_all(context):
    """Runs before all tests."""
    context.project_root = project_root


def before_feature(context, feature):
    """Runs before each feature."""
    pass


def before_scenario(context, scenario):
    """Runs before each scenario."""
    pass


def after_scenario(context, scenario):
    """Runs after each scenario."""
    # Restore environment variables
    if hasattr(context, "original_env") and context.original_env:
        for var_name, value in context.original_env.items():
            if value is None:
                os.environ.pop(var_name, None)
            else:
                os.environ[var_name] = value
    # Remove temporary directories created by steps
    if hasattr(context, "temp_dirs"):
        for path in context.temp_dirs:
            shutil.rmtree(path, ignore_errors=True)
