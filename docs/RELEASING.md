# Releasing a new version of SENTIL

Bump the version across every package, and update the changelog. Then, run the CI on main and make sure everything passes.

Next, run the `release-julia-artifacts` workflow on main with `prepare` ticked. It builds the Julia core tarballs, commits their hashes to `sentil-jl/Artifacts.toml` on main, and attaches the tarballs to a draft release for the version. The tag has to contain those hashes, so it goes on that commit.

Finally, pull that commit, tag it with `v` and the version, and push the tag. The tag workflows attach their files to the same draft. When they finish, `release-publish` writes `SHA256SUMS` over every file on the release and checks it against the release, the Julia hashes and the file names the READMEs mention. The draft is published only if all of them agree.