module.exports = {
  branches: [
    { name: "main" }
  ],
  plugins: [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    ["@google/semantic-release-replace-plugin", {
      replacements: [
        {
          files: ["Cargo.toml"],
          from: '^version = .*"$',
          to: 'version = "${nextRelease.version}"'
        }
      ]
    }],
    ["@semantic-release/exec", {
      shell: true,
      prepareCmd: `
        make in-docker TARGET='publish'
        make in-docker TARGET='publish' ARCH="aarch64"
      `
    }],
    ["@semantic-release/git", {
      assets: ["Cargo.toml", "Cargo.lock"],
      message: "chore(release): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}"
    }],
    ["@semantic-release/github", {
      assets: ["dist/*.rpm"],
    }]
  ]
};
