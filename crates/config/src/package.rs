// package.json

use moon_error::MoonError;
use moon_utils::fs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Person>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<Bin>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bugs: Option<Bug>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundled_dependencies: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributors: Option<Vec<Person>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Directories>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub engines: Option<EnginesSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding: Option<Funding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub man: Option<StringOrArray<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<OverridesSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies_meta: Option<PeerDepsMetaSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_config: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<ScriptsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub type_of: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspaces: Option<Workspaces>,

    // Node.js specific: https://nodejs.org/api/packages.html#nodejs-packagejson-field-definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,

    // Pnpm specific: https://pnpm.io/package_json
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnpm: Option<Pnpm>,

    // Yarn specific: https://yarnpkg.com/configuration/manifest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies_meta: Option<DepsMetaSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_config: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefer_unplugged: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolutions: Option<DepsSet>,

    // Unknown fields
    #[serde(flatten)]
    pub unknown_fields: BTreeMap<String, Value>,

    // Non-standard
    #[serde(skip)]
    pub path: PathBuf,
}

impl PackageJson {
    pub async fn load(path: &Path) -> Result<PackageJson, MoonError> {
        let mut cfg: PackageJson = fs::read_json(path).await?;
        cfg.path = path.to_path_buf();

        Ok(cfg)
    }

    pub async fn save(&self) -> Result<(), MoonError> {
        fs::write_json(&self.path, self, true).await?;

        Ok(())
    }

    pub fn add_dependency(&mut self, name: String, range: String, if_missing: bool) -> bool {
        let mut dependencies = match &self.dependencies {
            Some(deps) => deps.clone(),
            None => BTreeMap::new(),
        };

        // Only add if the dependency doesnt already exist
        if if_missing && dependencies.contains_key(&name) {
            return false;
        }

        dependencies.insert(name, range);

        self.dependencies = Some(dependencies);

        true
    }

    /// Add a version range to the `engines` field and return true if
    /// the new value is different from the old value.
    pub fn add_engine(&mut self, engine: &str, range: &str) -> bool {
        if let Some(engines) = &mut self.engines {
            if engines.contains_key(engine) && engines.get(engine).unwrap() == range {
                return false;
            }

            engines.insert(engine.to_owned(), range.to_owned());
        } else {
            self.engines = Some(BTreeMap::from([(engine.to_owned(), range.to_owned())]));
        }

        true
    }
}

pub type BinSet = BTreeMap<String, String>;
pub type DepsMetaSet = BTreeMap<String, DependencyMeta>;
pub type DepsSet = BTreeMap<String, String>;
pub type EnginesSet = BTreeMap<String, String>;
pub type OverridesSet = BTreeMap<String, StringOrObject<DepsSet>>;
pub type PeerDepsMetaSet = BTreeMap<String, PeerDependencyMeta>;
pub type ScriptsSet = BTreeMap<String, String>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StringOrArray<T> {
    String(String),
    Array(Vec<T>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StringOrObject<T> {
    String(String),
    Object(T),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StringArrayOrObject<T> {
    String(String),
    Array(Vec<T>),
    Object(T),
}

pub type Bin = StringOrArray<BinSet>;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Bug {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DependencyMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,

    // Yarn
    #[serde(skip_serializing_if = "Option::is_none")]
    pub built: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub unplugged: Option<bool>,

    // Pnpm
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injected: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Directories {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub man: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct FundingMetadata {
    #[serde(rename = "type")]
    pub type_of: String,
    pub url: String,
}

pub type Funding = StringArrayOrObject<FundingMetadata>;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LicenseMetadata {
    #[serde(rename = "type")]
    pub type_of: String,
    pub url: String,
}

pub type License = StringArrayOrObject<LicenseMetadata>;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PersonMetadata {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

pub type Person = StringOrObject<PersonMetadata>;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PeerDependencyMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Pnpm {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never_built_dependencies: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<OverridesSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_extensions: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RepositoryMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,

    #[serde(rename = "type")]
    pub type_of: String,

    pub url: String,
}

pub type Repository = StringOrObject<RepositoryMetadata>;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct WorkspacesExpanded {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nohoist: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Workspaces {
    Array(Vec<String>),
    Object(WorkspacesExpanded),
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_fs::prelude::*;

    #[tokio::test]
    async fn skips_none_when_writing() {
        let dir = assert_fs::TempDir::new().unwrap();
        let file = dir.child("package.json");
        file.write_str("{}").unwrap();

        let mut package = PackageJson::load(file.path()).await.unwrap();
        package.name = Some(String::from("hello"));
        package.description = Some(String::from("world"));
        package.keywords = Some(moon_utils::string_vec!["a", "b", "c"]);
        package.save().await.unwrap();

        let expected = serde_json::json!({
            "description": "world",
            "keywords": ["a", "b", "c"],
            "name": "hello",
        });

        assert_eq!(
            fs::read_json_string(file.path()).await.unwrap(),
            serde_json::to_string_pretty(&expected).unwrap(),
        );
    }

    // #[tokio::test]
    // async fn preserves_order_when_de_to_ser() {
    //     let json = r#"{"name": "hello", "description": "world", "private": true}"#;

    //     let dir = assert_fs::TempDir::new().unwrap();
    //     let file = dir.child("package.json");
    //     file.write_str(json).unwrap();

    //     let package = PackageJson::load(file.path()).await.unwrap();
    //     package.save().await.unwrap();

    //     assert_eq!(fs::read_json_string(file.path()).await.unwrap(), json,);
    // }
}
