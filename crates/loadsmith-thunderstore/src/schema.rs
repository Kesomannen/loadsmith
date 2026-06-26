use std::borrow::Cow;

use camino::Utf8PathBuf;
use loadsmith_install::{OwnedInstallRuleset, RouteRule};
use loadsmith_loader::BepInEx;
use thunderstore::models::schema;

use crate::Result;

pub fn r2_config_to_loader(
    config: &schema::R2ModmanConfig,
) -> Result<Option<Box<dyn loadsmith_loader::Loader>>> {
    match config.package_loader {
        schema::Loader::BepInEx => {
            let loader = BepInEx::with_rules(r2_config_to_ruleset(config)?);

            Ok(Some(Box::new(loader)))
        }
        _ => Ok(None),
    }
}

fn r2_config_to_ruleset(config: &schema::R2ModmanConfig) -> Result<OwnedInstallRuleset> {
    let rules = config
        .install_rules
        .iter()
        .map(|rule| rule_to_loadsmith(rule))
        .collect::<Result<Vec<_>>>()?;

    let default_rule_index = config
        .install_rules
        .iter()
        .position(|rule| rule.is_default_location);

    Ok(OwnedInstallRuleset::with_rules(rules, default_rule_index)
        .expect("default index should be in range"))
}

fn rule_to_loadsmith(rule: &schema::InstallRule) -> Result<loadsmith_install::InstallRule> {
    use schema::TrackingMethod;

    let name = rule
        .route
        .rsplit_once('/')
        .map(|(_, last)| last.to_string())
        .unwrap_or(rule.route.clone());

    let extensions = rule
        .default_file_extensions
        .iter()
        .map(|ext| {
            if ext.starts_with(".") {
                ext[1..].to_string()
            } else {
                ext.clone()
            }
        })
        .map(Cow::Owned)
        .collect();

    let subdir = matches!(
        rule.tracking_method,
        TrackingMethod::SubdirNoFlatten | TrackingMethod::Subdir
    );

    let flatten = matches!(
        rule.tracking_method,
        TrackingMethod::Subdir | TrackingMethod::PackageZip
    );

    let mutable = matches!(
        rule.tracking_method,
        TrackingMethod::State | TrackingMethod::None
    );

    let rule = loadsmith_install::InstallRule::Route(
        RouteRule::new(name, Utf8PathBuf::from(rule.route.clone()))
            .with_file_extensions(extensions)
            .with_subdir(subdir)
            .with_flatten(flatten)
            .with_mutable(mutable),
    );

    Ok(rule)
}

#[cfg(test)]
mod test {
    use super::*;

    use thunderstore::models::schema;

    #[test]
    fn parse_ecosystem() {
        let json = include_str!("../fixtures/ecosystem.json");
        let _ecosystem: schema::Schema =
            serde_json::from_str(json).expect("failed to parse ecosystem");
    }

    #[test]
    #[ignore]
    fn fetch_and_parse_ecosystem_live() {
        const URL: &str = "https://thunderstore.io/api/experimental/schema/dev/latest/";
        reqwest::blocking::get(URL)
            .expect("failed to fetch ecosystem")
            .error_for_status()
            .expect("ecosystem fetch returned error status")
            .json::<schema::Schema>()
            .expect("failed to parse ecosystem");
    }

    #[test]
    fn make_loader_lethal_company() {
        let json = include_str!("../fixtures/r2modman-lethal-company.json");
        let r2config: schema::R2ModmanConfig =
            serde_json::from_str(json).expect("failed to parse loader config");

        let loader = r2_config_to_loader(&r2config)
            .expect("failed to make loader")
            .expect("loader is None");

        insta::assert_debug_snapshot!(loader);
    }
}
