# synq

Perform a sync of upstream repositories onto local disk.

Similar to [`git-sync`](https://github.com/kubernetes/git-sync), but in Rust, much worse, and for my own personal use.


## Usage

```
synq path/to/config.toml
```

```toml
# example-config.toml

# Cloned repository will go into here
working_directory = "/tmp/synq-dirs"

[repo_details]
gruglb = {
  author = "jdockerty", repository = "gruglb", service = "github", post_sync = """
    echo 'my scripted action'
  """"
}
```

### Post sync actions

Post sync actions can be supplied as a regular shell script to the `post_sync` option of a repository.

This action assumes that `synq` is running in an environment with suitable permissions _or_ is only being supplied non-malicious actions.

