use tracing::{span, Span};

use crate::action::base::{CreateDirectory, CreateFile};
use crate::action::{Action, ActionDescription, ActionError, ActionTag, StatefulAction};

const NIX_CONF_FOLDER: &str = "/etc/nix";
const NIX_CONF: &str = "/etc/nix/nix.conf";

/**
Place the `/etc/nix.conf` file
 */
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PlaceNixConfiguration {
    create_directory: StatefulAction<CreateDirectory>,
    create_file: StatefulAction<CreateFile>,
}

impl PlaceNixConfiguration {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn plan(
        nix_build_group_name: String,
        extra_conf: Vec<String>,
        force: bool,
    ) -> Result<StatefulAction<Self>, ActionError> {
        let buf = format!(
            "\
            # Generated by https://github.com/DeterminateSystems/nix-installer, version {version}.\n\
            \n\
            {extra_conf}\n\
            \n\
            build-users-group = {nix_build_group_name}\n\
            \n\
            experimental-features = nix-command flakes\n\
            \n\
            auto-optimise-store = true\n\
            \n\
            bash-prompt-prefix = (nix:$name)\\040\n\
            \n\
            extra-nix-path = nixpkgs=flake:nixpkgs\n\
        ",
            extra_conf = extra_conf.join("\n"),
            version = env!("CARGO_PKG_VERSION"),
        );
        let create_directory = CreateDirectory::plan(NIX_CONF_FOLDER, None, None, 0o0755, force)
            .await
            .map_err(|e| ActionError::Child(CreateDirectory::action_tag(), Box::new(e)))?;
        let create_file = CreateFile::plan(NIX_CONF, None, None, 0o0664, buf, force)
            .await
            .map_err(|e| ActionError::Child(CreateFile::action_tag(), Box::new(e)))?;
        Ok(Self {
            create_directory,
            create_file,
        }
        .into())
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "place_nix_configuration")]
impl Action for PlaceNixConfiguration {
    fn action_tag() -> ActionTag {
        ActionTag("place_nix_configuration")
    }
    fn tracing_synopsis(&self) -> String {
        format!("Place the Nix configuration in `{NIX_CONF}`")
    }

    fn tracing_span(&self) -> Span {
        span!(tracing::Level::DEBUG, "place_nix_configuration",)
    }

    fn execute_description(&self) -> Vec<ActionDescription> {
        vec![ActionDescription::new(
            self.tracing_synopsis(),
            vec![
                "This file is read by the Nix daemon to set its configuration options at runtime."
                    .to_string(),
            ],
        )]
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn execute(&mut self) -> Result<(), ActionError> {
        self.create_directory
            .try_execute()
            .await
            .map_err(|e| ActionError::Child(self.create_directory.action_tag(), Box::new(e)))?;
        self.create_file
            .try_execute()
            .await
            .map_err(|e| ActionError::Child(self.create_file.action_tag(), Box::new(e)))?;

        Ok(())
    }

    fn revert_description(&self) -> Vec<ActionDescription> {
        vec![ActionDescription::new(
            format!("Remove the Nix configuration in `{NIX_CONF}`"),
            vec![
                "This file is read by the Nix daemon to set its configuration options at runtime."
                    .to_string(),
            ],
        )]
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn revert(&mut self) -> Result<(), ActionError> {
        self.create_file
            .try_revert()
            .await
            .map_err(|e| ActionError::Child(self.create_file.action_tag(), Box::new(e)))?;
        self.create_directory
            .try_revert()
            .await
            .map_err(|e| ActionError::Child(self.create_directory.action_tag(), Box::new(e)))?;

        Ok(())
    }
}
