import type { ElectrobunConfig } from "electrobun";

export default {
	app: {
		name: "ApiBuddy",
		identifier: "io.github.hezhangjian.apibuddy",
		version: "0.0.1",
	},
	build: {
		mainProcess: "cottontail",
		cottontail: {
			entrypoint: "src/main/index.ts",
		},
		// Vite builds to dist/, we copy from there
		copy: {
			"generated/cli-runtime.cjs": "cli/runtime.cjs",
			"dist/index.html": "views/mainview/index.html",
			"dist/assets": "views/mainview/assets",
		},
		// Ignore Vite output in watch mode — HMR handles view rebuilds separately
		watchIgnore: ["dist/**"],
		mac: {
			bundleCEF: false,
		},
		linux: {
			bundleCEF: false,
		},
		win: {
			bundleCEF: false,
		},
	},
} satisfies ElectrobunConfig;
