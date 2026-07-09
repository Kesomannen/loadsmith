use std::borrow::Cow;

use camino::Utf8Path;
use loadsmith_install::{OwnedInstallRuleset, RouteRule};
use loadsmith_loader::{BepInEx, MelonLoader};
use loadsmith_platform::Platform as LoadsmithPlatform;
use thunderstore::models::schema::{self, Distribution};

use crate::{Error, Result};

pub fn r2_config_to_loader(
    config: &schema::R2ModmanConfig,
) -> Result<Option<Box<dyn loadsmith_loader::Loader>>> {
    match config.package_loader {
        schema::Loader::BepInEx => {
            let loader = BepInEx::with_rules(convert_ruleset(&config.install_rules)?);

            Ok(Some(Box::new(loader)))
        }
        schema::Loader::MelonLoader => {
            let loader = MelonLoader::with_rules(convert_ruleset(&config.install_rules)?);

            Ok(Some(Box::new(loader)))
        }
        schema::Loader::RecursiveMelonLoader => {
            let mut loader = MelonLoader::with_default_recursive_rules();
            let ruleset = convert_ruleset(&config.install_rules)?;

            for rule in ruleset.into_rules() {
                loader.add_install_rule(rule);
            }

            Ok(Some(Box::new(loader)))
        }
        _ => Ok(None),
    }
}

fn convert_ruleset(rules: &[schema::InstallRule]) -> Result<OwnedInstallRuleset> {
    let default_rule_index = rules.iter().position(|rule| rule.is_default_location);
    let converted = rules
        .iter()
        .map(|rule| rule_to_loadsmith(rule, Utf8Path::new("")))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(
        OwnedInstallRuleset::with_rules(converted, default_rule_index)
            .expect("default index should be in range"),
    )
}

fn rule_to_loadsmith(
    rule: &schema::InstallRule,
    prefix: &Utf8Path,
) -> Result<Vec<loadsmith_install::InstallRule>> {
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
            if let Some(stripped) = ext.strip_prefix('.') {
                stripped.to_string()
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

    let target = prefix.join(&rule.route);
    let new_prefix = prefix.join(&name);

    let root_rule = loadsmith_install::InstallRule::Route(
        RouteRule::new_with_target(name, target)
            .with_file_extensions(extensions)
            .with_subdir(subdir)
            .with_flatten(flatten)
            .with_mutable(mutable),
    );

    let sub_rules = rule
        .sub_routes
        .iter()
        .map(|sub_route| rule_to_loadsmith(sub_route, &new_prefix))
        .collect::<Result<Vec<Vec<_>>>>()?;

    let rules = std::iter::once(root_rule)
        .chain(sub_rules.into_iter().flatten())
        .collect();

    Ok(rules)
}

pub fn distribution_into_platform(distribution: Distribution) -> Result<Option<LoadsmithPlatform>> {
    let identifier = distribution
        .identifier
        .ok_or_else(|| Error::DistributionIsMissingIdentifier);

    let platform = match distribution.platform {
        schema::Platform::Steam => {
            let id = identifier?.parse::<u32>().map_err(Error::InvalidSteamId)?;

            Ok(LoadsmithPlatform::Steam { id })
        }
        schema::Platform::EpicGamesStore => {
            identifier.map(|identifier| LoadsmithPlatform::EpicGames { identifier })
        }
        schema::Platform::XboxGamePass => {
            identifier.map(|identifier| LoadsmithPlatform::XboxStore { identifier })
        }
        schema::Platform::Other => Ok(LoadsmithPlatform::Other),
        schema::Platform::OculusStore => Ok(LoadsmithPlatform::Oculus),
        schema::Platform::Origin => Ok(LoadsmithPlatform::Origin),
        schema::Platform::SteamDirect => return Ok(None),
    }?;

    Ok(Some(platform))
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
        let response = reqwest::blocking::get(URL)
            .expect("failed to fetch ecosystem")
            .error_for_status()
            .expect("ecosystem fetch returned error status")
            .json::<schema::Schema>()
            .expect("failed to parse ecosystem");

        println!("{response:#?}");
    }

    fn assert_loader_snapshot(name: &'static str, json: &str) {
        let r2config: schema::R2ModmanConfig =
            serde_json::from_str(json).expect("failed to parse loader config");

        let loader = r2_config_to_loader(&r2config)
            .expect("failed to make loader")
            .expect("loader is None");

        insta::assert_debug_snapshot!(name, loader);
    }

    #[test]
    fn make_loader_lethal_company() {
        let json = include_str!("../fixtures/r2modman-lethal-company.json");
        assert_loader_snapshot("lethal-company", json);
    }

    #[test]
    fn make_loader_boneworks() {
        let json = include_str!("../fixtures/r2modman-boneworks.json");
        assert_loader_snapshot("boneworks", json);
    }

    #[test]
    fn make_loader_schedule_i() {
        let json = include_str!("../fixtures/r2modman-schedule-i.json");
        assert_loader_snapshot("schedule-i", json);
    }
}
