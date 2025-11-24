# -*- coding: utf-8 -*-
"""Tests for AWS client."""
import os
import pytest
from unittest.mock import MagicMock, patch, AsyncMock
import sys


class TestAWSConfig:
    """Tests for AWSConfig dataclass."""

    def test_from_env(self):
        with patch.dict(os.environ, {
            "AWS_ACCESS_KEY_ID": "test-access-key",
            "AWS_SECRET_ACCESS_KEY": "test-secret-key",
            "AWS_SESSION_TOKEN": "test-session-token",
            "AWS_DEFAULT_REGION": "us-west-2",
        }):
            from integrations.aws_client import AWSConfig
            config = AWSConfig.from_env()

            assert config.access_key_id == "test-access-key"
            assert config.secret_access_key == "test-secret-key"
            assert config.session_token == "test-session-token"
            assert config.region == "us-west-2"

    def test_from_env_defaults(self):
        # Save current env and set minimal required values
        with patch.dict(os.environ, {
            "AWS_ACCESS_KEY_ID": "key",
            "AWS_SECRET_ACCESS_KEY": "secret",
            "AWS_SESSION_TOKEN": "",
            "AWS_DEFAULT_REGION": "",
        }, clear=False):
            from integrations.aws_client import AWSConfig
            config = AWSConfig.from_env()

            assert config.access_key_id == "key"
            assert config.secret_access_key == "secret"

    def test_is_valid_true(self):
        from integrations.aws_client import AWSConfig
        config = AWSConfig(access_key_id="key", secret_access_key="secret")
        assert config.is_valid() is True

    def test_is_valid_false_no_key(self):
        from integrations.aws_client import AWSConfig
        config = AWSConfig(access_key_id="", secret_access_key="secret")
        assert config.is_valid() is False

    def test_is_valid_false_no_secret(self):
        from integrations.aws_client import AWSConfig
        config = AWSConfig(access_key_id="key", secret_access_key="")
        assert config.is_valid() is False

    def test_default_region(self):
        from integrations.aws_client import AWSConfig
        config = AWSConfig(access_key_id="key", secret_access_key="secret")
        assert config.region == "eu-central-1"


class TestAWSClient:
    """Tests for AWSClient class."""

    def test_init_no_boto3(self):
        """Test client init raises when boto3 not available."""
        from integrations.aws_client import AWSConfig

        with patch("integrations.aws_client.AWS_AVAILABLE", False):
            # Reimport to get the new value
            import importlib
            import integrations.aws_client as module
            original = module.AWS_AVAILABLE
            module.AWS_AVAILABLE = False

            with pytest.raises(ImportError, match="boto3"):
                from integrations.aws_client import AWSClient, AWSConfig
                AWSClient(config=AWSConfig(access_key_id="key", secret_access_key="secret"))

            module.AWS_AVAILABLE = original

    def test_init_invalid_credentials(self):
        """Test client init raises with invalid credentials."""
        from integrations.aws_client import AWSClient, AWSConfig, AWS_AVAILABLE

        if not AWS_AVAILABLE:
            pytest.skip("boto3 not installed")

        config = AWSConfig(access_key_id="", secret_access_key="")
        with pytest.raises(ValueError, match="credentials"):
            AWSClient(config=config)


class TestPrintResources:
    """Tests for print_resources function."""

    def test_print_resources_with_identity(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "identity": {"Account": "123456789012", "Arn": "arn:aws:iam::123456789012:user/test"},
            "ec2_instances": [{"id": "i-123", "name": "Test", "state": "running", "type": "t2.micro"}],
            "s3_buckets": [{"name": "bucket1"}],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "123456789012" in captured.out
        assert "i-123" in captured.out
        assert "bucket1" in captured.out

    def test_print_resources_empty(self, capsys):
        from integrations.aws_client import print_resources

        resources = {}
        print_resources(resources)
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_print_resources_lambda(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "lambda_functions": [{"name": "my-function", "runtime": "python3.9"}],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "Lambda Functions" in captured.out
        assert "my-function" in captured.out
        assert "python3.9" in captured.out

    def test_print_resources_dynamodb(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "dynamodb_tables": ["table1", "table2"],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "DynamoDB Tables" in captured.out
        assert "table1" in captured.out
        assert "table2" in captured.out

    def test_print_resources_rds(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "rds_instances": [{"id": "db-1", "engine": "mysql", "status": "available"}],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "RDS Instances" in captured.out
        assert "db-1" in captured.out
        assert "mysql" in captured.out

    def test_print_resources_ecs(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "ecs_clusters": [{
                "name": "my-cluster",
                "status": "ACTIVE",
                "active_services": 3,
                "running_tasks": 5
            }],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "ECS Clusters" in captured.out
        assert "my-cluster" in captured.out
        assert "ACTIVE" in captured.out

    def test_print_resources_eks(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "eks_clusters": [{"name": "k8s-cluster", "status": "ACTIVE", "version": "1.28"}],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "EKS Clusters" in captured.out
        assert "k8s-cluster" in captured.out

    def test_print_resources_cloudwatch(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "cloudwatch_log_groups": [{"name": "/aws/lambda/test", "retention_days": 14}],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "CloudWatch Log Groups" in captured.out
        assert "/aws/lambda/test" in captured.out

    def test_print_resources_sqs(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "sqs_queues": [{"url": "https://sqs.region.amazonaws.com/123/queue", "approx_messages": 10, "inflight": 2}],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "SQS Queues" in captured.out
        assert "messages: 10" in captured.out

    def test_print_resources_sns(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "sns_topics": [{"arn": "arn:aws:sns:region:123:my-topic"}],
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "SNS Topics" in captured.out
        assert "my-topic" in captured.out

    def test_print_resources_costs(self, capsys):
        from integrations.aws_client import print_resources

        resources = {
            "cost_estimates": {"Amazon EC2": 100.50, "Amazon S3": 25.75},
        }

        print_resources(resources)
        captured = capsys.readouterr()
        assert "Cost Estimates" in captured.out
        assert "$100.50" in captured.out
        assert "$25.75" in captured.out


class TestAWSAvailable:
    """Tests for AWS_AVAILABLE flag."""

    def test_aws_available_defined(self):
        from integrations.aws_client import AWS_AVAILABLE
        assert isinstance(AWS_AVAILABLE, bool)
