use async_graphql::SimpleObject;

use crate::aws::codecommit::{
    CodeCommitBranchInfo, CodeCommitPullRequestInfo, CodeCommitPullRequestTargetInfo,
    CodeCommitRepositoryInfo,
};

#[derive(SimpleObject, Clone)]
pub struct CodeCommitRepository {
    pub repository_id: Option<String>,
    pub repository_name: Option<String>,
    pub repository_description: Option<String>,
    pub default_branch: Option<String>,
    pub last_modified_date: Option<String>,
    pub creation_date: Option<String>,
    pub clone_url_http: Option<String>,
    pub clone_url_ssh: Option<String>,
    pub arn: Option<String>,
}

impl From<CodeCommitRepositoryInfo> for CodeCommitRepository {
    fn from(r: CodeCommitRepositoryInfo) -> Self {
        Self {
            repository_id: r.repository_id,
            repository_name: r.repository_name,
            repository_description: r.repository_description,
            default_branch: r.default_branch,
            last_modified_date: r.last_modified_date,
            creation_date: r.creation_date,
            clone_url_http: r.clone_url_http,
            clone_url_ssh: r.clone_url_ssh,
            arn: r.arn,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CodeCommitBranch {
    pub branch_name: Option<String>,
    pub commit_id: Option<String>,
}

impl From<CodeCommitBranchInfo> for CodeCommitBranch {
    fn from(b: CodeCommitBranchInfo) -> Self {
        Self {
            branch_name: b.branch_name,
            commit_id: b.commit_id,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CodeCommitPullRequestTarget {
    pub repository_name: Option<String>,
    pub source_reference: Option<String>,
    pub destination_reference: Option<String>,
    pub merge_base: Option<String>,
}

impl From<CodeCommitPullRequestTargetInfo> for CodeCommitPullRequestTarget {
    fn from(t: CodeCommitPullRequestTargetInfo) -> Self {
        Self {
            repository_name: t.repository_name,
            source_reference: t.source_reference,
            destination_reference: t.destination_reference,
            merge_base: t.merge_base,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct CodeCommitPullRequest {
    pub pull_request_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub pull_request_status: Option<String>,
    pub author_arn: Option<String>,
    pub creation_date: Option<String>,
    pub last_activity_date: Option<String>,
    pub targets: Vec<CodeCommitPullRequestTarget>,
}

impl From<CodeCommitPullRequestInfo> for CodeCommitPullRequest {
    fn from(p: CodeCommitPullRequestInfo) -> Self {
        Self {
            pull_request_id: p.pull_request_id,
            title: p.title,
            description: p.description,
            pull_request_status: p.pull_request_status,
            author_arn: p.author_arn,
            creation_date: p.creation_date,
            last_activity_date: p.last_activity_date,
            targets: p
                .targets
                .into_iter()
                .map(CodeCommitPullRequestTarget::from)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::codecommit::{
        CodeCommitBranchInfo, CodeCommitPullRequestInfo, CodeCommitPullRequestTargetInfo,
        CodeCommitRepositoryInfo,
    };

    #[test]
    fn test_repository_from_full() {
        let info = CodeCommitRepositoryInfo {
            repository_id: Some("abc-123".to_string()),
            repository_name: Some("my-repo".to_string()),
            repository_description: Some("Test repo".to_string()),
            default_branch: Some("main".to_string()),
            last_modified_date: Some("2024-01-15T10:00:00Z".to_string()),
            creation_date: Some("2024-01-01T00:00:00Z".to_string()),
            clone_url_http: Some("https://git-codecommit.us-east-1.amazonaws.com/v1/repos/my-repo".to_string()),
            clone_url_ssh: Some("ssh://git-codecommit.us-east-1.amazonaws.com/v1/repos/my-repo".to_string()),
            arn: Some("arn:aws:codecommit:us-east-1:123456789012:my-repo".to_string()),
        };
        let result = CodeCommitRepository::from(info);
        assert_eq!(result.repository_id, Some("abc-123".to_string()));
        assert_eq!(result.repository_name, Some("my-repo".to_string()));
        assert_eq!(result.default_branch, Some("main".to_string()));
        assert!(result.clone_url_http.is_some());
        assert!(result.arn.is_some());
    }

    #[test]
    fn test_repository_from_minimal() {
        let info = CodeCommitRepositoryInfo {
            repository_id: None,
            repository_name: Some("sparse-repo".to_string()),
            repository_description: None,
            default_branch: None,
            last_modified_date: None,
            creation_date: None,
            clone_url_http: None,
            clone_url_ssh: None,
            arn: None,
        };
        let result = CodeCommitRepository::from(info);
        assert_eq!(result.repository_name, Some("sparse-repo".to_string()));
        assert!(result.default_branch.is_none());
        assert!(result.arn.is_none());
    }

    #[test]
    fn test_branch_from() {
        let info = CodeCommitBranchInfo {
            branch_name: Some("main".to_string()),
            commit_id: Some("abc123def456".to_string()),
        };
        let result = CodeCommitBranch::from(info);
        assert_eq!(result.branch_name, Some("main".to_string()));
        assert_eq!(result.commit_id, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_branch_from_no_commit() {
        let info = CodeCommitBranchInfo {
            branch_name: Some("feature/xyz".to_string()),
            commit_id: None,
        };
        let result = CodeCommitBranch::from(info);
        assert_eq!(result.branch_name, Some("feature/xyz".to_string()));
        assert!(result.commit_id.is_none());
    }

    #[test]
    fn test_pull_request_target_from() {
        let info = CodeCommitPullRequestTargetInfo {
            repository_name: Some("my-repo".to_string()),
            source_reference: Some("refs/heads/feature/xyz".to_string()),
            destination_reference: Some("refs/heads/main".to_string()),
            merge_base: Some("abc123".to_string()),
        };
        let result = CodeCommitPullRequestTarget::from(info);
        assert_eq!(result.repository_name, Some("my-repo".to_string()));
        assert_eq!(result.source_reference, Some("refs/heads/feature/xyz".to_string()));
        assert_eq!(result.destination_reference, Some("refs/heads/main".to_string()));
        assert_eq!(result.merge_base, Some("abc123".to_string()));
    }

    #[test]
    fn test_pull_request_from_full() {
        let info = CodeCommitPullRequestInfo {
            pull_request_id: Some("1".to_string()),
            title: Some("Add feature X".to_string()),
            description: Some("This PR adds feature X".to_string()),
            pull_request_status: Some("OPEN".to_string()),
            author_arn: Some("arn:aws:iam::123456789012:user/jdoe".to_string()),
            creation_date: Some("2024-01-15T10:00:00Z".to_string()),
            last_activity_date: Some("2024-01-16T12:00:00Z".to_string()),
            targets: vec![CodeCommitPullRequestTargetInfo {
                repository_name: Some("my-repo".to_string()),
                source_reference: Some("refs/heads/feature/x".to_string()),
                destination_reference: Some("refs/heads/main".to_string()),
                merge_base: None,
            }],
        };
        let result = CodeCommitPullRequest::from(info);
        assert_eq!(result.pull_request_id, Some("1".to_string()));
        assert_eq!(result.pull_request_status, Some("OPEN".to_string()));
        assert_eq!(result.targets.len(), 1);
        assert_eq!(result.targets[0].repository_name, Some("my-repo".to_string()));
    }

    #[test]
    fn test_pull_request_from_no_targets() {
        let info = CodeCommitPullRequestInfo {
            pull_request_id: Some("2".to_string()),
            title: Some("Fix bug".to_string()),
            description: None,
            pull_request_status: Some("CLOSED".to_string()),
            author_arn: None,
            creation_date: None,
            last_activity_date: None,
            targets: vec![],
        };
        let result = CodeCommitPullRequest::from(info);
        assert_eq!(result.pull_request_id, Some("2".to_string()));
        assert_eq!(result.pull_request_status, Some("CLOSED".to_string()));
        assert!(result.targets.is_empty());
        assert!(result.description.is_none());
    }
}
