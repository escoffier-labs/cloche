use std::process::ExitCode;

use schemars::schema_for;

use crate::cli::SchemaArgs;
use crate::cli::SchemaTarget;
use crate::contract::AppshotResult;

pub(crate) fn schema(args: SchemaArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let schema = schema_value(args.contract);
    if args.compact {
        println!("{}", serde_json::to_string(&schema)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    }
    Ok(ExitCode::SUCCESS)
}

fn schema_value(target: SchemaTarget) -> serde_json::Value {
    let schema = match target {
        SchemaTarget::Capture => schema_for!(AppshotResult),
        SchemaTarget::Polish => schema_for!(crate::contract::PolishResult),
        SchemaTarget::Config => schema_for!(crate::contract::ConfigResult),
    };
    serde_json::to_value(schema).expect("schema serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_for_capture_describes_the_capture_contract() {
        let value = schema_value(SchemaTarget::Capture);
        let properties = value["properties"].as_object().expect("properties");
        assert!(properties.contains_key("outDir"));
        assert!(
            !properties.contains_key("outputDir"),
            "legacy outputDir must not appear in the published schema"
        );
        assert!(properties.contains_key("backend"));
        assert!(!properties.contains_key("card"));
    }

    #[test]
    fn schema_for_polish_describes_the_polish_contract() {
        let value = schema_value(SchemaTarget::Polish);
        let properties = value["properties"].as_object().expect("properties");
        assert!(properties.contains_key("card"));
        assert!(properties.contains_key("presentationStyle"));
        assert!(!properties.contains_key("backend"));
    }

    #[test]
    fn schema_for_config_describes_the_config_contract() {
        let value = schema_value(SchemaTarget::Config);
        let properties = value["properties"].as_object().expect("properties");
        assert!(properties.contains_key("config"));
        assert!(properties.contains_key("options"));
        assert!(properties.contains_key("path"));
    }
}
