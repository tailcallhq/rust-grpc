use gh_workflow::ctx::Context;
use gh_workflow::toolchain::Toolchain;
use gh_workflow::*;

#[test]
fn generate() {
    let flags = RustFlags::deny("warnings");

    let build = Job::new("Build and Test")
        .permissions(Permissions::default().contents(Level::Read))
        .add_step(Step::checkout())
        .add_step(Step::uses("arduino", "setup-protoc", "3"))
        .add_step(
            Toolchain::default()
                .add_stable()
                .add_nightly()
                .add_clippy()
                .add_fmt(),
        )
        .add_step(
            Cargo::new("test")
                .args("--all-features --workspace")
                .name("Cargo Test"),
        )
        .add_step(
            Cargo::new("fmt")
                .nightly()
                .args("--check")
                .name("Cargo Fmt"),
        )
        .add_step(
            Cargo::new("clippy")
                .nightly()
                .args("--all-features --workspace -- -D warnings")
                .name("Cargo Clippy"),
        );

    let event = Event::default()
        .push(Push::default().add_branch("main"))
        .pull_request(
            PullRequest::default()
                .add_type(PullRequestType::Opened)
                .add_type(PullRequestType::Synchronize)
                .add_type(PullRequestType::Reopened)
                .add_type(PullRequestType::Closed)
                .add_branch("main"),
        );

    let deploy = Job::new("Deploy to Shuttle")
        .permissions(Permissions::default().contents(Level::Write))
        .cond(
            Context::github()
                .event_name()
                .eq("push".into())
                .and(Context::github().ref_().eq("refs/heads/main".into())),
        )
        .runs_on("ubuntu-latest")
        .add_step(Step::checkout())
        .add_step(
            Step::uses("shuttle-hq/deploy-action", "deploy-action", "main")
                .add_with(("deploy-key", "${{ secrets.SHUTTLE_API_KEY }}")),
        );

    Workflow::new("Build and Test")
        .add_env(flags)
        .on(event)
        .add_job("build", build)
        .add_job("deploy", deploy)
        .generate()
        .unwrap();
}
