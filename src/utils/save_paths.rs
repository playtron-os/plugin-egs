use regex::Regex;

pub fn expand_path_variables(path: String, user_id: String) -> String {
    let regex = Regex::new(r"\{(\w+)\}").unwrap();
    let mut output = path.clone();
    let captures = regex.captures_iter(&path);
    let mut offset = 0_usize;
    for capture in captures {
        if let Some(match_group) = capture.get(1) {
            let path_id = match_group.as_str().to_lowercase();
            let res = match path_id.as_str() {
                "installdir" => String::from("{INSTALL}"),
                "epicid" => user_id.clone(),
                "appdata" => String::from("C:/Users/{USER}/AppData/Local"),
                "userdir" => String::from("C:/Users/{USER}/Documents"),
                "userprofile" => String::from("C:/Users/{USER}"),
                "usersavedgames" => String::from("C:/Users/{USER}/Saved Games"),
                _ => String::default(),
            };

            if !res.is_empty() {
                let diff = res.len() - (path_id.len() + 2);
                let start = match_group.start() + offset - 1;
                let end = match_group.end() + offset + 1;
                output.replace_range(start..end, &res);
                offset += diff;
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use crate::utils::save_paths::expand_path_variables;

    const TEST_ACCOUNT_ID: &str = "ACCOUNT_ID";
    #[test]
    fn expand_absolute_drift() {
        assert_eq!(
            expand_path_variables(
                "{AppData}/../LocalLow/Funselektor Labs Inc_/Absolute Drift/EOS_cloud".to_string(),
                TEST_ACCOUNT_ID.to_string()
            ),
            "C:/Users/{USER}/AppData/Local/../LocalLow/Funselektor Labs Inc_/Absolute Drift/EOS_cloud"
        );
    }

    #[test]
    fn expand_death_stranding() {
        assert_eq!(
            expand_path_variables(
                "{AppData}/../LocalLow/KojimaProductions/DeathStrandingDC/{EpicID}/".to_string(),
                TEST_ACCOUNT_ID.to_string()
            ),
            format!("C:/Users/{{USER}}/AppData/Local/../LocalLow/KojimaProductions/DeathStrandingDC/{TEST_ACCOUNT_ID}/")
        );
    }

    #[test]
    fn expand_ghostwire_tokio() {
        assert_eq!(
            expand_path_variables(
                "{UserSavedGames}/TangoGameworks/GhostWire Tokyo (EGS)/Saved/".to_string(),
                TEST_ACCOUNT_ID.to_string()
            ),
            "C:/Users/{USER}/Saved Games/TangoGameworks/GhostWire Tokyo (EGS)/Saved/"
        )
    }
}
