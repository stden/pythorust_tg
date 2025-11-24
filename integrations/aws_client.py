# -*- coding: utf-8 -*-
"""
AWS utilities for listing and managing cloud resources.

Supports:
- EC2 instances
- S3 buckets
- Lambda functions
- DynamoDB tables
- RDS databases
- IAM users/roles
"""

import os
from typing import Optional, List, Dict, Any
from dataclasses import dataclass
from dotenv import load_dotenv

load_dotenv()

# TODO: Implement cost estimation for resources
# TODO: Add resource tagging and filtering

try:
    import boto3
    from botocore.exceptions import ClientError, NoCredentialsError
    AWS_AVAILABLE = True
except ImportError:
    AWS_AVAILABLE = False
    boto3 = None
    ClientError = None
    NoCredentialsError = None


@dataclass
class AWSConfig:
    """AWS configuration from environment."""
    access_key_id: str
    secret_access_key: str
    session_token: Optional[str] = None
    region: str = "eu-central-1"

    @classmethod
    def from_env(cls) -> "AWSConfig":
        """Load AWS config from environment variables."""
        return cls(
            access_key_id=os.getenv("AWS_ACCESS_KEY_ID", ""),
            secret_access_key=os.getenv("AWS_SECRET_ACCESS_KEY", ""),
            session_token=os.getenv("AWS_SESSION_TOKEN"),
            region=os.getenv("AWS_DEFAULT_REGION", "eu-central-1"),
        )

    def is_valid(self) -> bool:
        """Check if credentials are set."""
        return bool(self.access_key_id and self.secret_access_key)


class AWSClient:
    """AWS client wrapper for common operations."""

    def __init__(self, config: Optional[AWSConfig] = None):
        if not AWS_AVAILABLE:
            raise ImportError("boto3 not installed. Run: pip install boto3")

        self.config = config or AWSConfig.from_env()
        if not self.config.is_valid():
            raise ValueError("AWS credentials not configured")

        self._session = boto3.Session(
            aws_access_key_id=self.config.access_key_id,
            aws_secret_access_key=self.config.secret_access_key,
            aws_session_token=self.config.session_token,
            region_name=self.config.region,
        )

    def get_caller_identity(self) -> Dict[str, str]:
        """Get current AWS identity."""
        sts = self._session.client("sts")
        return sts.get_caller_identity()

    # === EC2 ===

    def list_ec2_instances(self, filters: Optional[List[Dict]] = None) -> List[Dict[str, Any]]:
        """List EC2 instances with optional filters."""
        ec2 = self._session.client("ec2")
        params = {}
        if filters:
            params["Filters"] = filters

        response = ec2.describe_instances(**params)
        instances = []
        for reservation in response.get("Reservations", []):
            for instance in reservation.get("Instances", []):
                name = ""
                for tag in instance.get("Tags", []):
                    if tag["Key"] == "Name":
                        name = tag["Value"]
                        break

                instances.append({
                    "id": instance["InstanceId"],
                    "name": name,
                    "type": instance.get("InstanceType"),
                    "state": instance["State"]["Name"],
                    "public_ip": instance.get("PublicIpAddress"),
                    "private_ip": instance.get("PrivateIpAddress"),
                    "launch_time": str(instance.get("LaunchTime")),
                })
        return instances

    # === S3 ===

    def list_s3_buckets(self) -> List[Dict[str, Any]]:
        """List S3 buckets."""
        s3 = self._session.client("s3")
        response = s3.list_buckets()
        return [
            {
                "name": bucket["Name"],
                "created": str(bucket["CreationDate"]),
            }
            for bucket in response.get("Buckets", [])
        ]

    def list_s3_objects(self, bucket: str, prefix: str = "", max_keys: int = 100) -> List[Dict[str, Any]]:
        """List objects in S3 bucket."""
        s3 = self._session.client("s3")
        response = s3.list_objects_v2(Bucket=bucket, Prefix=prefix, MaxKeys=max_keys)
        return [
            {
                "key": obj["Key"],
                "size": obj["Size"],
                "modified": str(obj["LastModified"]),
            }
            for obj in response.get("Contents", [])
        ]

    # === Lambda ===

    def list_lambda_functions(self) -> List[Dict[str, Any]]:
        """List Lambda functions."""
        lambda_client = self._session.client("lambda")
        response = lambda_client.list_functions()
        return [
            {
                "name": fn["FunctionName"],
                "runtime": fn.get("Runtime"),
                "memory": fn.get("MemorySize"),
                "timeout": fn.get("Timeout"),
                "last_modified": fn.get("LastModified"),
            }
            for fn in response.get("Functions", [])
        ]

    # === DynamoDB ===

    def list_dynamodb_tables(self) -> List[str]:
        """List DynamoDB tables."""
        dynamodb = self._session.client("dynamodb")
        response = dynamodb.list_tables()
        return response.get("TableNames", [])

    # === RDS ===

    def list_rds_instances(self) -> List[Dict[str, Any]]:
        """List RDS database instances."""
        rds = self._session.client("rds")
        response = rds.describe_db_instances()
        return [
            {
                "id": db["DBInstanceIdentifier"],
                "engine": db["Engine"],
                "status": db["DBInstanceStatus"],
                "class": db["DBInstanceClass"],
                "endpoint": db.get("Endpoint", {}).get("Address"),
            }
            for db in response.get("DBInstances", [])
        ]

    # === IAM ===

    def list_iam_users(self) -> List[Dict[str, Any]]:
        """List IAM users."""
        iam = self._session.client("iam")
        response = iam.list_users()
        return [
            {
                "name": user["UserName"],
                "id": user["UserId"],
                "created": str(user["CreateDate"]),
            }
            for user in response.get("Users", [])
        ]

    def list_iam_roles(self) -> List[Dict[str, Any]]:
        """List IAM roles."""
        iam = self._session.client("iam")
        response = iam.list_roles()
        return [
            {
                "name": role["RoleName"],
                "id": role["RoleId"],
                "created": str(role["CreateDate"]),
            }
            for role in response.get("Roles", [])
        ]

    # === ECS ===

    def list_ecs_clusters(self) -> List[Dict[str, Any]]:
        """List ECS clusters with basic stats."""
        ecs = self._session.client("ecs")
        arns = ecs.list_clusters().get("clusterArns", [])
        if not arns:
            return []

        clusters: List[Dict[str, Any]] = []
        # describe_clusters accepts up to 100 arns
        for i in range(0, len(arns), 100):
            chunk = arns[i:i + 100]
            response = ecs.describe_clusters(clusters=chunk)
            for cluster in response.get("clusters", []):
                clusters.append({
                    "arn": cluster["clusterArn"],
                    "name": cluster.get("clusterName"),
                    "status": cluster.get("status"),
                    "active_services": cluster.get("activeServicesCount"),
                    "running_tasks": cluster.get("runningTasksCount"),
                })
        return clusters

    # === EKS ===

    def list_eks_clusters(self) -> List[Dict[str, Any]]:
        """List EKS clusters."""
        eks = self._session.client("eks")
        names = eks.list_clusters().get("clusters", [])
        clusters = []
        for name in names:
            details = eks.describe_cluster(name=name).get("cluster", {})
            clusters.append({
                "name": name,
                "status": details.get("status"),
                "version": details.get("version"),
                "endpoint": details.get("endpoint"),
            })
        return clusters

    # === CloudWatch Logs ===

    def list_cloudwatch_log_groups(self, limit: int = 50) -> List[Dict[str, Any]]:
        """List CloudWatch log groups."""
        logs = self._session.client("logs")
        response = logs.describe_log_groups(limit=limit)
        return [
            {
                "name": group["logGroupName"],
                "stored_bytes": group.get("storedBytes"),
                "retention_days": group.get("retentionInDays"),
            }
            for group in response.get("logGroups", [])
        ]

    # === SQS ===

    def list_sqs_queues(self) -> List[Dict[str, Any]]:
        """List SQS queues with basic metrics."""
        sqs = self._session.client("sqs")
        urls = sqs.list_queues().get("QueueUrls", [])
        queues = []
        for url in urls:
            attrs = sqs.get_queue_attributes(
                QueueUrl=url,
                AttributeNames=[
                    "QueueArn",
                    "ApproximateNumberOfMessages",
                    "ApproximateNumberOfMessagesNotVisible",
                ],
            ).get("Attributes", {})
            queues.append({
                "url": url,
                "arn": attrs.get("QueueArn"),
                "approx_messages": int(attrs.get("ApproximateNumberOfMessages", 0)),
                "inflight": int(attrs.get("ApproximateNumberOfMessagesNotVisible", 0)),
            })
        return queues

    # === SNS ===

    def list_sns_topics(self) -> List[Dict[str, Any]]:
        """List SNS topics."""
        sns = self._session.client("sns")
        response = sns.list_topics()
        return [
            {
                "arn": topic["TopicArn"],
            }
            for topic in response.get("Topics", [])
        ]

    # === Resource Cleanup ===

    def terminate_ec2_instances(self, instance_ids: List[str], wait: bool = False) -> List[str]:
        """Terminate EC2 instances by ID."""
        ec2 = self._session.client("ec2")
        response = ec2.terminate_instances(InstanceIds=instance_ids)
        terminated_ids = [item["InstanceId"] for item in response.get("TerminatingInstances", [])]
        if wait and terminated_ids:
            waiter = ec2.get_waiter("instance_terminated")
            waiter.wait(InstanceIds=terminated_ids)
        return terminated_ids

    def empty_s3_bucket(self, bucket: str, prefix: str = "") -> int:
        """Delete all objects (and versions) from an S3 bucket."""
        s3 = self._session.client("s3")
        deleted = 0

        version_paginator = s3.get_paginator("list_object_versions")
        for page in version_paginator.paginate(Bucket=bucket, Prefix=prefix):
            objects = [{"Key": obj["Key"], "VersionId": obj["VersionId"]} for obj in page.get("Versions", [])]
            objects += [{"Key": marker["Key"], "VersionId": marker["VersionId"]} for marker in page.get("DeleteMarkers", [])]
            if not objects:
                continue
            for idx in range(0, len(objects), 1000):
                chunk = objects[idx:idx + 1000]
                resp = s3.delete_objects(Bucket=bucket, Delete={"Objects": chunk, "Quiet": True})
                deleted += len(resp.get("Deleted", []))

        object_paginator = s3.get_paginator("list_objects_v2")
        for page in object_paginator.paginate(Bucket=bucket, Prefix=prefix):
            objects = [{"Key": obj["Key"]} for obj in page.get("Contents", [])]
            if not objects:
                continue
            for idx in range(0, len(objects), 1000):
                chunk = objects[idx:idx + 1000]
                resp = s3.delete_objects(Bucket=bucket, Delete={"Objects": chunk, "Quiet": True})
                deleted += len(resp.get("Deleted", []))

        return deleted

    def delete_s3_bucket(self, bucket: str, force: bool = True) -> bool:
        """Delete an S3 bucket, optionally emptying it first."""
        s3 = self._session.client("s3")
        if force:
            self.empty_s3_bucket(bucket)
        s3.delete_bucket(Bucket=bucket)
        return True

    def delete_lambda_function(self, function_name: str) -> bool:
        """Delete a Lambda function."""
        lambda_client = self._session.client("lambda")
        lambda_client.delete_function(FunctionName=function_name)
        return True

    def delete_dynamodb_table(self, table_name: str, wait: bool = False) -> bool:
        """Delete a DynamoDB table."""
        dynamodb = self._session.client("dynamodb")
        dynamodb.delete_table(TableName=table_name)
        if wait:
            waiter = dynamodb.get_waiter("table_not_exists")
            waiter.wait(TableName=table_name)
        return True

    def delete_rds_instance(
        self,
        db_identifier: str,
        skip_final_snapshot: bool = True,
        delete_automated_backups: bool = True,
        final_snapshot_identifier: Optional[str] = None,
        wait: bool = False,
    ) -> bool:
        """Delete an RDS instance with optional snapshot creation."""
        rds = self._session.client("rds")
        params: Dict[str, Any] = {
            "DBInstanceIdentifier": db_identifier,
            "SkipFinalSnapshot": skip_final_snapshot,
            "DeleteAutomatedBackups": delete_automated_backups,
        }
        if not skip_final_snapshot and final_snapshot_identifier:
            params["FinalDBSnapshotIdentifier"] = final_snapshot_identifier

        rds.delete_db_instance(**params)

        if wait:
            waiter = rds.get_waiter("db_instance_deleted")
            waiter.wait(DBInstanceIdentifier=db_identifier)

        return True

    # === All Resources Summary ===

    def get_all_resources(self) -> Dict[str, Any]:
        """Get summary of all AWS resources."""
        resources = {}

        try:
            resources["identity"] = self.get_caller_identity()
        except Exception as e:
            resources["identity_error"] = str(e)

        try:
            resources["ec2_instances"] = self.list_ec2_instances()
        except Exception as e:
            resources["ec2_error"] = str(e)

        try:
            resources["s3_buckets"] = self.list_s3_buckets()
        except Exception as e:
            resources["s3_error"] = str(e)

        try:
            resources["lambda_functions"] = self.list_lambda_functions()
        except Exception as e:
            resources["lambda_error"] = str(e)

        try:
            resources["dynamodb_tables"] = self.list_dynamodb_tables()
        except Exception as e:
            resources["dynamodb_error"] = str(e)

        try:
            resources["rds_instances"] = self.list_rds_instances()
        except Exception as e:
            resources["rds_error"] = str(e)

        try:
            resources["ecs_clusters"] = self.list_ecs_clusters()
        except Exception as e:
            resources["ecs_error"] = str(e)

        try:
            resources["eks_clusters"] = self.list_eks_clusters()
        except Exception as e:
            resources["eks_error"] = str(e)

        try:
            resources["cloudwatch_log_groups"] = self.list_cloudwatch_log_groups()
        except Exception as e:
            resources["cloudwatch_error"] = str(e)

        try:
            resources["sqs_queues"] = self.list_sqs_queues()
        except Exception as e:
            resources["sqs_error"] = str(e)

        try:
            resources["sns_topics"] = self.list_sns_topics()
        except Exception as e:
            resources["sns_error"] = str(e)

        return resources


def print_resources(resources: Dict[str, Any]) -> None:
    """Print AWS resources in readable format."""
    if "identity" in resources:
        print(f"\n=== AWS Identity ===")
        identity = resources["identity"]
        print(f"Account: {identity.get('Account')}")
        print(f"User ARN: {identity.get('Arn')}")

    if "ec2_instances" in resources:
        instances = resources["ec2_instances"]
        print(f"\n=== EC2 Instances ({len(instances)}) ===")
        for inst in instances:
            print(f"  {inst['id']}: {inst['name']} ({inst['state']}) - {inst['type']}")

    if "s3_buckets" in resources:
        buckets = resources["s3_buckets"]
        print(f"\n=== S3 Buckets ({len(buckets)}) ===")
        for bucket in buckets:
            print(f"  {bucket['name']}")

    if "lambda_functions" in resources:
        functions = resources["lambda_functions"]
        print(f"\n=== Lambda Functions ({len(functions)}) ===")
        for fn in functions:
            print(f"  {fn['name']} ({fn['runtime']})")

    if "dynamodb_tables" in resources:
        tables = resources["dynamodb_tables"]
        print(f"\n=== DynamoDB Tables ({len(tables)}) ===")
        for table in tables:
            print(f"  {table}")

    if "rds_instances" in resources:
        instances = resources["rds_instances"]
        print(f"\n=== RDS Instances ({len(instances)}) ===")
        for db in instances:
            print(f"  {db['id']} ({db['engine']}) - {db['status']}")

    if "ecs_clusters" in resources:
        clusters = resources["ecs_clusters"]
        print(f"\n=== ECS Clusters ({len(clusters)}) ===")
        for cluster in clusters:
            print(f"  {cluster['name']} ({cluster['status']}) - services: {cluster['active_services']}, running tasks: {cluster['running_tasks']}")

    if "eks_clusters" in resources:
        clusters = resources["eks_clusters"]
        print(f"\n=== EKS Clusters ({len(clusters)}) ===")
        for cluster in clusters:
            print(f"  {cluster['name']} ({cluster['status']}) v{cluster.get('version')}")

    if "cloudwatch_log_groups" in resources:
        groups = resources["cloudwatch_log_groups"]
        print(f"\n=== CloudWatch Log Groups ({len(groups)}) ===")
        for group in groups:
            print(f"  {group['name']} - retention: {group.get('retention_days')}")

    if "sqs_queues" in resources:
        queues = resources["sqs_queues"]
        print(f"\n=== SQS Queues ({len(queues)}) ===")
        for queue in queues:
            print(f"  {queue['url']} - messages: {queue['approx_messages']} inflight: {queue['inflight']}")

    if "sns_topics" in resources:
        topics = resources["sns_topics"]
        print(f"\n=== SNS Topics ({len(topics)}) ===")
        for topic in topics:
            print(f"  {topic['arn']}")


async def main():
    """Example usage."""
    try:
        client = AWSClient()
        print("Getting AWS resources...")
        resources = client.get_all_resources()
        print_resources(resources)
    except ValueError as e:
        print(f"Error: {e}")
        print("Please configure AWS credentials in .env file")
    except Exception as e:
        print(f"AWS Error: {e}")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
