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
  author = "jdockerty", repository = "gruglb", service = "github"
}
```
