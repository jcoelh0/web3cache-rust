use k8s_openapi::api::{
    apps::v1::{Deployment, DeploymentSpec},
    core::v1::ContainerPort,
};

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{Api, PostParams};
use serde_json::{json, Value};

use kube::Client;

use log::error;
use mongodb::{
    bson::{doc, Document},
    Database,
};

use serde::{Deserialize, Serialize};

pub struct AppState {
    pub db: Database,
    pub environment: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NftResp {
    pub data: Vec<Document>,
    pub offset: u64,
}

#[derive(Deserialize)]
pub struct ContractNftInfo {
    pub contract_address: String,
    pub owner: Option<bool>,
    pub limit: u64,
    pub offset: u64,
    pub metadata: Option<bool>,
}

#[derive(Deserialize)]
pub struct OnwerNftInfo {
    pub address: String,
    pub contract_address: Vec<String>,
    pub limit: u64,
    pub offset: u64,
    pub metadata: Option<bool>,
}

#[derive(Deserialize)]
pub struct Info {
    pub contract_id: Option<String>,
    pub address: String,
}

/* pub async fn check_api_key(req: &HttpRequest, db: Database) -> anyhow::Result<bool> {
    let api_key = req.headers().get("x-read-api-key");

    if api_key.is_none() {
        return Ok(false);
    }
    let api_key = api_key.unwrap().to_str().unwrap().to_string();

    let find_option = FindOneOptions::default();
    //find_option.projection = Some(doc! { "apikey": 0, "secret": 0, "__v": 0  });

    let api_key_document: Option<Document> = db
        .collection("apikeys")
        .find_one(doc! { "apikey" : api_key }, find_option)
        .await
        .unwrap();

    if api_key_document.is_some() {
        Ok(true)
    } else {
        Ok(false)
    }
} */

// Define a global Client object
/* lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::try_default()
        .await.expect("Failed to create client");
} */

pub fn deployment_to_contractid(deployment: &String) -> String {
    deployment[16..deployment.len()]
        .to_string()
        .replace('-', "_")
}

pub fn contractid_to_deployment(contract_id: &str) -> String {
    format!("web3cache-write-{}", contract_id.replace('_', "-"))
}

pub async fn get_write_deployments() -> anyhow::Result<Vec<String>> {
    let client: Client = Client::try_default()
        .await
        .expect("Failed to create client");

    let deployments_api: Api<Deployment> = Api::namespaced(client, "web3cache");

    // Set list params to limit to only running pods
    let lp = Default::default(); //ListParams::default().fields("status.phase=Running");

    // Use the Pod API to list all running pods in the cluster
    let deployment_list = deployments_api.list(&lp).await?;

    let deployment_names = deployment_list
        .items
        .into_iter()
        .filter_map(|deployment| {
            if deployment
                .metadata
                .name
                .as_ref()
                .unwrap()
                .contains("web3cache-write")
            {
                let name = deployment.metadata.name.unwrap();
                Some(name)
            } else {
                None
            }
        })
        .collect::<Vec<String>>();

    //info!("{:?}", deployment_names);

    Ok(deployment_names)
}

pub async fn add_write_deployment(
    contract_id: String,
    environment: Vec<Value>,
) -> anyhow::Result<()> {
    let client: Client = Client::try_default()
        .await
        .expect("Failed to create client");

    let deployment_name = contractid_to_deployment(&contract_id);

    let namespace = "web3cache";

    let mut labels = std::collections::BTreeMap::new();
    labels.insert("component".to_owned(), "web3cache-write".to_owned());

    let mut env = vec![json!({
        "name": "CONTRACTID",
        "value": contract_id
    })];
    env.extend_from_slice(&environment);

    let container_ports: Vec<ContainerPort> = vec![];

    let container = json!({
        "name": "web3cache-write",
        "image": "ghcr.io/orangecomet/web3cache-write:dev", // TODO
        "imagePullPolicy": "Always",
        "env": env,
        "resources": {
            "requests": {
                "memory": "140Mi",
                "cpu": "40m"
            },
            "limits": {
                "memory": "200Mi"
            }
        },
        "ports": container_ports,
        "volumeMounts": [
        {
            "name": "secrets-store-inline",
            "mountPath": "/mnt/secrets-store",
            "readOnly": true
        }
    ]
    });

    let mut template_labels = std::collections::BTreeMap::new();
    template_labels.insert("component".to_owned(), "web3cache-write".to_owned());

    let pod_template_spec = json!({
        "metadata": {
            "labels": template_labels
        },
        "spec": {
            "terminationGracePeriodSeconds": 5,
            "containers": [container],
            "imagePullSecrets": [{
                "name": "dockerconfigjson-github-com"
            }],
            "volumes": [
                {
                    "name": "secrets-store-inline",
                    "csi": {
                        "driver": "secrets-store.csi.k8s.io",
                        "readOnly": true,
                        "volumeAttributes": {
                            "secretProviderClass": "web3cache-write"
                        }
                    }
                }
            ]
        }
    });

    let deployment_spec = DeploymentSpec {
        replicas: Some(1),
        selector: k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector {
            match_labels: Some(labels.clone()),
            ..Default::default()
        },
        template: serde_json::from_value(pod_template_spec).unwrap(),
        ..Default::default()
    };

    let deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(deployment_name.clone()),
            namespace: Some(namespace.to_owned()),
            ..Default::default()
        },
        spec: Some(deployment_spec),
        ..Default::default()
    };

    let deployments: Api<Deployment> = Api::namespaced(client, namespace);

    let pp = PostParams::default();
    let result = deployments.create(&pp, &deployment).await;
    match result {
        Ok(_deployment) => {
            /* println!(
                "Created deployment {} in namespace {}",
                deployment_name, namespace
            ); */
        }
        Err(kube::Error::Api(e)) if e.code == 409 => {
            error!(
                "Deployment {} in namespace {} already exists",
                deployment_name, namespace
            );
        }
        Err(e) => {
            error!(
                "Error creating deployment {} in namespace {}: {}",
                deployment_name, namespace, e
            );
        }
    }

    Ok(())
}

pub async fn delete_write_deployments(deployment_name: String) -> anyhow::Result<()> {
    let client: Client = Client::try_default()
        .await
        .expect("Failed to create client");

    let deployments_api: Api<Deployment> = Api::namespaced(client, "web3cache");

    let delete_params = Default::default();

    match deployments_api
        .delete(&deployment_name, &delete_params)
        .await
    {
        Ok(_deployment_deleted) => {
            //info!("Deployment deleted: {:?}", deployment_deleted);
        }
        Err(e) => {
            error!("Error deleting deployment: {:?}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_to_contractid() {
        let deployment = String::from("web3cache-write-abcdef123456-7890");
        let contract_id = deployment_to_contractid(&deployment);

        assert_eq!("abcdef123456_7890", contract_id);
    }

    #[test]
    fn test_contractid_to_deployment() {
        let contract_id = String::from("abcdef123456_7890");
        let deployment = contractid_to_deployment(&contract_id);

        assert_eq!("web3cache-write-abcdef123456-7890", deployment);
    }

    #[test]
    fn test_conversion_consistency() {
        let original_deployment = String::from("web3cache-write-abcdef123456-7890");

        let contract_id = deployment_to_contractid(&original_deployment);
        let reconstructed_deployment = contractid_to_deployment(&contract_id);

        assert_eq!(original_deployment, reconstructed_deployment);

        let original_contract_id = String::from("abcdef123456_7890");

        let deployment = contractid_to_deployment(&original_contract_id);
        let reconstructed_contract_id = deployment_to_contractid(&deployment);

        assert_eq!(original_contract_id, reconstructed_contract_id);
    }
}
