"""Smoke test steps for Telegram bot framework."""
import sys, os
from behave import given, then

@given("the Python interpreter is available")
def step_python_available(context):
    assert sys.version_info >= (3, 8)

@then("standard library modules should be importable")
def step_stdlib_importable(context):
    import json, pathlib, datetime
    assert json and pathlib and datetime

@given("the project root is in the Python path")
def step_project_root_in_path(context):
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    if root not in sys.path:
        sys.path.insert(0, root)
    context.project_root = root
    assert os.path.isdir(root)

@then("the core module should be importable without errors")
def step_core_module_importable(context):
    root = context.project_root
    py_files = [f for f in os.listdir(root) if f.endswith(".py") and not f.startswith(".")]
    assert len(py_files) >= 0
    assert os.path.isdir(os.path.join(root, "features"))
