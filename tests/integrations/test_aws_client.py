"""Tests for AWS client integration."""

import pytest
from unittest.mock import MagicMock, patch
import os
from datetime import date, timedelta

from integrations.aws_client import (
    AWSConfig,
    AWSClient,
    EC2Instance,
    S3Bucket,
    LambdaFunction,
    DynamoDBTable,
    RDSDatabase,
    IAMUser,
    IAMRole,
    AWS_AVAILABLE,
)


class TestAWSConfig:
    """Test AWSConfig class."""

    def test_from_env_with_all_values(self, monkeypatch):
        """Test loading config from environment with all values."""
        monkeypatch.setenv("AWS_ACCESS_KEY_ID", "test_key")
        monkeypatch.setenv("AWS_SECRET_ACCESS_KEY", "test_secret")
        monkeypatch.setenv("AWS_SESSION_TOKEN", "test_token")
        monkeypatch.setenv("AWS_DEFAULT_REGION", "us-west-2")

        config = AWSConfig.from_env()

        assert config.access_key_id == "test_key"
        assert config.secret_access_key == "test_secret"
        assert config.session_token == "test_token"
        assert config.region == "us-west-2"

    def test_from_env_with_defaults(self, monkeypatch):
        """Test loading config with default values."""
        monkeypatch.delenv("AWS_SESSION_TOKEN", raising=False)
        monkeypatch.delenv("AWS_DEFAULT_REGION", raising=False)

        config = AWSConfig.from_env()

        assert config.session_token is None
        assert config.region == "eu-central-1"


@pytest.mark.skipif(not AWS_AVAILABLE, reason="AWS SDK not available")
class TestAWSClient:
    """Test AWSClient class."""

    @pytest.fixture
    def mock_boto_session(self):
        """Mock boto3 session."""
        with patch("integrations.aws_client.boto3") as mock_boto3:
            session = MagicMock()
            mock_boto3.Session.return_value = session
            yield session

    @pytest.fixture
    def aws_client(self):
        """Create AWS client instance."""
        config = AWSConfig(
            access_key_id="test_key",
            secret_access_key="test_secret",
            region="us-east-1"
        )
        return AWSClient(config)

    def test_init(self, aws_client):
        """Test client initialization."""
        assert aws_client.config.access_key_id == "test_key"
        assert aws_client.config.region == "us-east-1"
        assert aws_client._session is None

    def test_session_lazy_loading(self, aws_client, mock_boto_session):
        """Test lazy loading of boto3 session."""
        # First access should create session
        session1 = aws_client.session
        assert session1 is not None
        
        # Second access should return same session
        session2 = aws_client.session
        assert session2 is session1

    def test_list_ec2_instances(self, aws_client, mock_boto_session):
        """Test listing EC2 instances."""
        # Mock EC2 client
        ec2_client = MagicMock()
        mock_boto_session.client.return_value = ec2_client

        # Mock response
        ec2_client.describe_instances.return_value = {
            "Reservations": [
                {
                    "Instances": [
                        {
                            "InstanceId": "i-123456",
                            "InstanceType": "t2.micro",
                            "State": {"Name": "running"},
                            "LaunchTime": date.today(),
                            "PublicIpAddress": "1.2.3.4",
                            "PrivateIpAddress": "10.0.0.1",
                            "Tags": [{"Key": "Name", "Value": "test-instance"}],
                        }
                    ]
                }
            ]
        }

        instances = aws_client.list_ec2_instances()

        assert len(instances) == 1
        instance = instances[0]
        assert instance.instance_id == "i-123456"
        assert instance.instance_type == "t2.micro"
        assert instance.state == "running"
        assert instance.name == "test-instance"

    def test_list_s3_buckets(self, aws_client, mock_boto_session):
        """Test listing S3 buckets."""
        # Mock S3 client
        s3_client = MagicMock()
        mock_boto_session.client.return_value = s3_client

        # Mock response
        s3_client.list_buckets.return_value = {
            "Buckets": [
                {
                    "Name": "test-bucket",
                    "CreationDate": date.today(),
                }
            ]
        }
        s3_client.get_bucket_location.return_value = {"LocationConstraint": "us-east-1"}
        s3_client.list_objects_v2.return_value = {"KeyCount": 42}

        buckets = aws_client.list_s3_buckets()

        assert len(buckets) == 1
        bucket = buckets[0]
        assert bucket.name == "test-bucket"
        assert bucket.region == "us-east-1"
        assert bucket.object_count == 42

    def test_list_lambda_functions(self, aws_client, mock_boto_session):
        """Test listing Lambda functions."""
        # Mock Lambda client
        lambda_client = MagicMock()
        mock_boto_session.client.return_value = lambda_client

        # Mock response
        lambda_client.list_functions.return_value = {
            "Functions": [
                {
                    "FunctionName": "test-function",
                    "FunctionArn": "arn:aws:lambda:us-east-1:123456789:function:test-function",
                    "Runtime": "python3.9",
                    "Handler": "index.handler",
                    "CodeSize": 1024,
                    "MemorySize": 128,
                    "Timeout": 30,
                    "LastModified": "2025-01-01T00:00:00Z",
                }
            ]
        }

        functions = aws_client.list_lambda_functions()

        assert len(functions) == 1
        function = functions[0]
        assert function.name == "test-function"
        assert function.runtime == "python3.9"
        assert function.memory_size == 128

    def test_list_dynamodb_tables(self, aws_client, mock_boto_session):
        """Test listing DynamoDB tables."""
        # Mock DynamoDB client
        dynamodb_client = MagicMock()
        mock_boto_session.client.return_value = dynamodb_client

        # Mock response
        dynamodb_client.list_tables.return_value = {"TableNames": ["test-table"]}
        dynamodb_client.describe_table.return_value = {
            "Table": {
                "TableName": "test-table",
                "TableStatus": "ACTIVE",
                "ItemCount": 100,
                "TableSizeBytes": 1024,
                "CreationDateTime": date.today(),
                "KeySchema": [{"AttributeName": "id", "KeyType": "HASH"}],
            }
        }

        tables = aws_client.list_dynamodb_tables()

        assert len(tables) == 1
        table = tables[0]
        assert table.name == "test-table"
        assert table.status == "ACTIVE"
        assert table.item_count == 100

    def test_list_rds_databases(self, aws_client, mock_boto_session):
        """Test listing RDS databases."""
        # Mock RDS client
        rds_client = MagicMock()
        mock_boto_session.client.return_value = rds_client

        # Mock response
        rds_client.describe_db_instances.return_value = {
            "DBInstances": [
                {
                    "DBInstanceIdentifier": "test-db",
                    "DBInstanceClass": "db.t3.micro",
                    "Engine": "postgres",
                    "DBInstanceStatus": "available",
                    "Endpoint": {"Address": "test-db.region.rds.amazonaws.com"},
                    "AllocatedStorage": 20,
                    "MasterUsername": "admin",
                }
            ]
        }

        databases = aws_client.list_rds_databases()

        assert len(databases) == 1
        db = databases[0]
        assert db.identifier == "test-db"
        assert db.engine == "postgres"
        assert db.status == "available"

    def test_list_iam_users(self, aws_client, mock_boto_session):
        """Test listing IAM users."""
        # Mock IAM client
        iam_client = MagicMock()
        mock_boto_session.client.return_value = iam_client

        # Mock response
        iam_client.list_users.return_value = {
            "Users": [
                {
                    "UserName": "test-user",
                    "UserId": "AIDACKCEVSQ6C2EXAMPLE",
                    "Arn": "arn:aws:iam::123456789:user/test-user",
                    "Path": "/",
                    "CreateDate": date.today(),
                }
            ]
        }
        iam_client.list_access_keys.return_value = {
            "AccessKeyMetadata": [{"AccessKeyId": "AKIAIOSFODNN7EXAMPLE"}]
        }
        iam_client.list_attached_user_policies.return_value = {
            "AttachedPolicies": [{"PolicyName": "AdministratorAccess"}]
        }

        users = aws_client.list_iam_users()

        assert len(users) == 1
        user = users[0]
        assert user.user_name == "test-user"
        assert user.access_key_count == 1
        assert "AdministratorAccess" in user.policies

    def test_list_iam_roles(self, aws_client, mock_boto_session):
        """Test listing IAM roles."""
        # Mock IAM client
        iam_client = MagicMock()
        mock_boto_session.client.return_value = iam_client

        # Mock response
        iam_client.list_roles.return_value = {
            "Roles": [
                {
                    "RoleName": "test-role",
                    "RoleId": "AROACLKWSDQRAOEXAMPLE",
                    "Arn": "arn:aws:iam::123456789:role/test-role",
                    "Path": "/",
                    "CreateDate": date.today(),
                    "AssumeRolePolicyDocument": '{"Version": "2012-10-17"}',
                }
            ]
        }
        iam_client.list_attached_role_policies.return_value = {
            "AttachedPolicies": [{"PolicyName": "PowerUserAccess"}]
        }

        roles = aws_client.list_iam_roles()

        assert len(roles) == 1
        role = roles[0]
        assert role.role_name == "test-role"
        assert "PowerUserAccess" in role.policies

    def test_client_error_handling(self, aws_client, mock_boto_session):
        """Test handling of AWS client errors."""
        # Mock EC2 client with error
        ec2_client = MagicMock()
        mock_boto_session.client.return_value = ec2_client
        
        from botocore.exceptions import ClientError
        ec2_client.describe_instances.side_effect = ClientError(
            {"Error": {"Code": "UnauthorizedOperation", "Message": "Not authorized"}},
            "DescribeInstances"
        )

        with pytest.raises(ClientError):
            aws_client.list_ec2_instances()

    def test_no_credentials_error(self, aws_client, mock_boto_session):
        """Test handling when no credentials are available."""
        from botocore.exceptions import NoCredentialsError
        mock_boto_session.client.side_effect = NoCredentialsError()

        with pytest.raises(NoCredentialsError):
            aws_client.list_ec2_instances()


class TestAWSClientWithoutSDK:
    """Test behavior when AWS SDK is not available."""

    @patch("integrations.aws_client.AWS_AVAILABLE", False)
    def test_init_without_sdk(self):
        """Test that client cannot be initialized without SDK."""
        config = AWSConfig(
            access_key_id="test",
            secret_access_key="test"
        )
        
        with pytest.raises(ImportError):
            AWSClient(config)