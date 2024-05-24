use std::{env, error::Error, fs::File, io::{BufWriter, Write}, path::Path};

pub async fn check_env_vars() -> Result<String, Box<dyn Error>> {
    let exe_path = env::current_exe()?;
    let exe_parent = match exe_path.parent() {
        Some(parent_dir) => parent_dir.display(),
        None => return Err("Could not get parent directory of executable.".into()),
    };

    let env_path = format!("{}/variables.env", exe_parent);
    let env_path = Path::new(&env_path);
    if !env_path.exists() {
        let env_file = File::create(env_path)?;
        let mut env_writer = BufWriter::new(env_file);
        env_writer.write_all(b"SENTRY_DSN=\n")?;
        env_writer.write_all(b"PY_ENV_ACTIVATE_PATH=/home/user/env-name/bin/activate\n")?;
        env_writer.write_all(b"YOUTUBE_API_KEY=myyoutubeapikey\n")?;
        env_writer.write_all(b"UPSTASH_REDIS_ENDPOINT=us1-adjective-animal...\n")?;
        env_writer.write_all(b"UPSTASH_REDIS_PASSWORD=12345\n")?;

        return Err("variables.env not found. Generated template variables.env for user input.".into())
    }

    dotenv::from_path(env_path).ok();

    Ok(exe_parent.to_string())
}

