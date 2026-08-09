import eslint from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(eslint.configs.recommended, ...tseslint.configs.recommended, {
	ignores: ["dist", "public/config.js", "src/routeTree.gen.ts"],
});
