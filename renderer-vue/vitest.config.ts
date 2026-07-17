import viteConfig from "./vite.config.js";

export default {
	...viteConfig,
	test: {
		environment: "happy-dom",
		include: ["tests/**/*.test.ts"],
		clearMocks: true,
		restoreMocks: true,
	},
};
