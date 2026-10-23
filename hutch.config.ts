// @hutch cottontail=0.6.0
export default {
	scripts: {
		install: ["hutch", "install", "--frozen-lockfile"],
		start: ["node", "scripts/desktop.mjs", "dev"],
		dev: ["node", "scripts/desktop.mjs", "dev"],
		build: ["node", "scripts/desktop.mjs", "stable"],
		check: ["pnpm", "exec", "tsc", "--noEmit"],
		test: ["node", "scripts/test.mjs"],
	},
	electrobun: {
		version: "2.0.1",
	},
};
