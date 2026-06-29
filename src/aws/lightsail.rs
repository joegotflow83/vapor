use aws_config::SdkConfig;

use crate::error::VaporError;

pub struct LightsailInstanceInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub blueprint_id: Option<String>,
    pub bundle_id: Option<String>,
    pub state: Option<String>,
    pub public_ip_address: Option<String>,
    pub private_ip_address: Option<String>,
    pub location: Option<String>,
    pub created_at: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct LightsailEndpointInfo {
    pub port: i32,
    pub address: String,
}

pub struct LightsailDatabaseInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
    pub state: Option<String>,
    pub master_username: Option<String>,
    pub master_endpoint: Option<LightsailEndpointInfo>,
    pub created_at: Option<String>,
    pub tags: Vec<(String, String)>,
}

pub struct LightsailInstanceHealthInfo {
    pub instance_name: Option<String>,
    pub instance_health: Option<String>,
}

pub struct LightsailLoadBalancerInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub dns_name: Option<String>,
    pub state: Option<String>,
    pub protocol: Option<String>,
    pub instance_port: Option<i32>,
    pub instance_health_summary: Vec<LightsailInstanceHealthInfo>,
    pub created_at: Option<String>,
}

pub struct LightsailStaticIpInfo {
    pub name: Option<String>,
    pub arn: Option<String>,
    pub ip_address: Option<String>,
    pub attached_to: Option<String>,
    pub is_attached: bool,
}

pub struct LightsailClient {
    inner: aws_sdk_lightsail::Client,
}

impl LightsailClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            inner: aws_sdk_lightsail::Client::new(config),
        }
    }

    pub async fn get_instances(&self) -> Result<Vec<LightsailInstanceInfo>, VaporError> {
        let mut items = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.inner.get_instances();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for inst in output.instances() {
                items.push(LightsailInstanceInfo {
                    name: inst.name().map(|s| s.to_string()),
                    arn: inst.arn().map(|s| s.to_string()),
                    blueprint_id: inst.blueprint_id().map(|s| s.to_string()),
                    bundle_id: inst.bundle_id().map(|s| s.to_string()),
                    state: inst.state().and_then(|s| s.name()).map(|s| s.to_string()),
                    public_ip_address: inst.public_ip_address().map(|s| s.to_string()),
                    private_ip_address: inst.private_ip_address().map(|s| s.to_string()),
                    location: inst
                        .location()
                        .and_then(|l| l.availability_zone())
                        .map(|s| s.to_string()),
                    created_at: inst.created_at().map(|dt| dt.to_string()),
                    tags: inst
                        .tags()
                        .iter()
                        .map(|t| {
                            (
                                t.key().unwrap_or_default().to_string(),
                                t.value().unwrap_or_default().to_string(),
                            )
                        })
                        .collect(),
                });
            }
            match output.next_page_token() {
                Some(tok) if !tok.is_empty() => page_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_relational_databases(&self) -> Result<Vec<LightsailDatabaseInfo>, VaporError> {
        let mut items = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.inner.get_relational_databases();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for db in output.relational_databases() {
                items.push(LightsailDatabaseInfo {
                    name: db.name().map(|s| s.to_string()),
                    arn: db.arn().map(|s| s.to_string()),
                    engine: db.engine().map(|s| s.to_string()),
                    engine_version: db.engine_version().map(|s| s.to_string()),
                    state: db.state().map(|s| s.to_string()),
                    master_username: db.master_username().map(|s| s.to_string()),
                    master_endpoint: db.master_endpoint().map(|ep| LightsailEndpointInfo {
                        port: ep.port().unwrap_or(0),
                        address: ep.address().unwrap_or_default().to_string(),
                    }),
                    created_at: db.created_at().map(|dt| dt.to_string()),
                    tags: db
                        .tags()
                        .iter()
                        .map(|t| {
                            (
                                t.key().unwrap_or_default().to_string(),
                                t.value().unwrap_or_default().to_string(),
                            )
                        })
                        .collect(),
                });
            }
            match output.next_page_token() {
                Some(tok) if !tok.is_empty() => page_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_load_balancers(&self) -> Result<Vec<LightsailLoadBalancerInfo>, VaporError> {
        let mut items = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.inner.get_load_balancers();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for lb in output.load_balancers() {
                items.push(LightsailLoadBalancerInfo {
                    name: lb.name().map(|s| s.to_string()),
                    arn: lb.arn().map(|s| s.to_string()),
                    dns_name: lb.dns_name().map(|s| s.to_string()),
                    state: lb.state().map(|s| s.as_str().to_string()),
                    protocol: lb.protocol().map(|s| s.as_str().to_string()),
                    instance_port: lb.instance_port().map(|p| p as i32),
                    instance_health_summary: lb
                        .instance_health_summary()
                        .iter()
                        .map(|h| LightsailInstanceHealthInfo {
                            instance_name: h.instance_name().map(|s| s.to_string()),
                            instance_health: h
                                .instance_health()
                                .map(|s| s.as_str().to_string()),
                        })
                        .collect(),
                    created_at: lb.created_at().map(|dt| dt.to_string()),
                });
            }
            match output.next_page_token() {
                Some(tok) if !tok.is_empty() => page_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }

    pub async fn get_static_ips(&self) -> Result<Vec<LightsailStaticIpInfo>, VaporError> {
        let mut items = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.inner.get_static_ips();
            if let Some(ref tok) = page_token {
                req = req.page_token(tok);
            }
            let output = req.send().await.map_err(|e| VaporError::AwsSdk(e.to_string()))?;
            for ip in output.static_ips() {
                items.push(LightsailStaticIpInfo {
                    name: ip.name().map(|s| s.to_string()),
                    arn: ip.arn().map(|s| s.to_string()),
                    ip_address: ip.ip_address().map(|s| s.to_string()),
                    attached_to: ip.attached_to().map(|s| s.to_string()),
                    is_attached: ip.is_attached().unwrap_or(false),
                });
            }
            match output.next_page_token() {
                Some(tok) if !tok.is_empty() => page_token = Some(tok.to_string()),
                _ => break,
            }
        }

        Ok(items)
    }
}
