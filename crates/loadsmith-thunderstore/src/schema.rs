use std::borrow::Cow;

use camino::Utf8Path;
use globset::{Glob, GlobSet};
use loadsmith_install::{OwnedInstallRuleset, RouteRule};
use loadsmith_loader::{
    BepInEx, GDWeave, Lovely, MelonLoader, Northstar, ReturnOfModding, Rivet, Shimloader,
};
use loadsmith_platform::Platform as LoadsmithPlatform;
use thunderstore::models::schema::{self, Distribution};
use tracing::warn;

use crate::{Error, Result};

/// Converts an r2modman loader configuration into a [`Loader`](loadsmith_loader::Loader) implementation.
///
/// Maps the thunderstore loader enum to the corresponding loadsmith loader
/// type (BepInEx, MelonLoader, GDWeave, etc.) and applies any install rules
/// specified in the config.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::r2_config_to_loader;
/// use thunderstore::models::schema::R2ModmanConfig;
///
/// let json = r#"{
///     "meta": {"displayName": "Test", "iconUrl": null},
///     "internalFolderName": "test",
///     "dataFolderName": "test",
///     "distributions": [],
///     "settingsIdentifier": "test",
///     "packageIndex": "",
///     "steamFolderName": "",
///     "exeNames": [],
///     "gameInstanceType": "",
///     "gameSelectionDisplayMode": "",
///     "additionalSearchStrings": [],
///     "packageLoader": "bepinex",
///     "installRules": []
/// }"#;
/// let config: R2ModmanConfig = serde_json::from_str(json).unwrap();
/// let loader = r2_config_to_loader(&config).unwrap();
/// ```
pub fn r2_config_to_loader(
    config: &schema::R2ModmanConfig,
) -> Result<Box<dyn loadsmith_loader::Loader>> {
    match config.package_loader {
        schema::Loader::BepInEx => {
            let loader = BepInEx::with_rules(convert_ruleset(config)?);
            Ok(Box::new(loader))
        }
        schema::Loader::MelonLoader => {
            let loader = MelonLoader::with_rules(convert_ruleset(config)?);
            Ok(Box::new(loader))
        }
        schema::Loader::RecursiveMelonLoader => {
            let mut loader = MelonLoader::with_default_recursive_rules();

            let (rules, default, exclude) = convert_ruleset(config)?.into_parts();

            if let Some(exclude) = exclude {
                loader.set_exclude(exclude);
            }
            if let Some(default) = default {
                warn!(
                    "loader config specifies a default rule index, but the recursive melon loader does not support default rules; ignoring default rule index {default}"
                );
            }
            for rule in rules {
                loader.add_install_rule(rule);
            }

            Ok(Box::new(loader))
        }
        schema::Loader::Shimloader => {
            let loader = Shimloader::with_rules(convert_ruleset(config)?);
            Ok(Box::new(loader))
        }
        schema::Loader::ReturnOfModding => {
            let loader = ReturnOfModding::with_rules(convert_ruleset(config)?);
            Ok(Box::new(loader))
        }
        schema::Loader::BepisLoader => {
            let loader = BepInEx::with_rules(convert_ruleset(config)?);
            Ok(Box::new(loader))
        }
        schema::Loader::GDWeave => {
            warn_install_rules_not_supported("GDWeave", &config.install_rules);

            Ok(Box::new(GDWeave::new()))
        }
        schema::Loader::Lovely => {
            warn_install_rules_not_supported("Lovely", &config.install_rules);

            Ok(Box::new(Lovely::new()))
        }
        schema::Loader::Rivet => {
            warn_install_rules_not_supported("Rivet", &config.install_rules);

            Ok(Box::new(Rivet::new()))
        }
        schema::Loader::Northstar => {
            let loader = Northstar::with_rules(convert_ruleset(config)?);

            Ok(Box::new(loader))
        }
        loader => Err(Error::UnsupportedLoader(loader)),
    }
}

fn warn_install_rules_not_supported(loader_name: &str, install_rules: &[schema::InstallRule]) {
    if !install_rules.is_empty() {
        warn!(
            "loader config specifies install rules, but {loader_name} does not support custom install rules; ignoring rules"
        );
    }
}

fn convert_ruleset(config: &schema::R2ModmanConfig) -> Result<OwnedInstallRuleset> {
    let default_rule_index = config
        .install_rules
        .iter()
        .position(|rule| rule.is_default_location);

    let converted_rules = config
        .install_rules
        .iter()
        .map(|rule| rule_to_loadsmith(rule, Utf8Path::new("")))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    let exclude_glob_set = match &config.relative_file_exclusions {
        Some(exclusions) => {
            let globs = exclusions
                .iter()
                .map(|glob| Glob::new(glob).map_err(Error::Glob))
                .collect::<Result<Vec<_>>>()?;

            Some(GlobSet::new(globs)?)
        }
        None => None,
    };

    let mut ruleset = OwnedInstallRuleset::with_rules(converted_rules, default_rule_index)
        .expect("default index should be in range");

    if let Some(exclude) = exclude_glob_set {
        ruleset.set_exclude(exclude);
    }

    Ok(ruleset)
}

fn rule_to_loadsmith(
    rule: &schema::InstallRule,
    prefix: &Utf8Path,
) -> Result<Vec<loadsmith_install::InstallRule>> {
    use schema::TrackingMethod;

    if rule.tracking_method == TrackingMethod::PackageZip {
        return Err(Error::UnsupportedTrackingMethod(rule.tracking_method));
    }

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
        TrackingMethod::Subdir | TrackingMethod::None | TrackingMethod::PackageZip
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

/// Converts a thunderstore [`Distribution`] into a [`loadsmith_platform::Platform`].
///
/// Handles Steam (with numeric ID), Epic Games, Xbox Game Pass, Oculus,
/// Origin, and generic platforms.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::distribution_into_platform;
/// use thunderstore::models::schema::Distribution;
///
/// let json = r#"{"platform": "steam", "identifier": "12345"}"#;
/// let dist: Distribution = serde_json::from_str(json).unwrap();
/// let platform = distribution_into_platform(dist).unwrap();
/// assert!(matches!(platform, loadsmith_platform::Platform::Steam { id: 12345 }));
/// ```
pub fn distribution_into_platform(distribution: Distribution) -> Result<LoadsmithPlatform> {
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
        platform => return Err(Error::UnsupportedPlatform(platform)),
    }?;

    Ok(platform)
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

        let loader = r2_config_to_loader(&r2config).expect("failed to make loader");

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

    #[test]
    fn make_loader_voices_of_the_void() {
        let json = include_str!("../fixtures/r2modman-voices-of-the-void.json");
        assert_loader_snapshot("voices-of-the-void", json);
    }
}
