module.exports = {
  // ESLint looks recursively in all parent directories for configs and combines
  // them. This stops it looking in directories above this one.
  root: true,

  // So `module` etc. are allowed.
  env: {
    node: true,
  },

  plugins: [
    "@typescript-eslint",
  ],

  parserOptions: {
    ecmaVersion: 2020,
    sourceType: "module",
  },

  // These enable a default set of rules that are tweaked below. Note that
  // the order here is important - each config is applied in order and can
  // override the previous ones.
  extends: [
    // Default set of recommended rules (these have a tick mark next to them
    // on the ESLint website).
    "eslint:recommended",

    // This config then disables rules from eslint:recommended that are
    // already handled by the Typescript compiler itself.
    "plugin:@typescript-eslint/eslint-recommended",

    // Enable recommended Typescript ESLint config.
    "plugin:@typescript-eslint/recommended",
  ],

  rules: {
    "no-empty": "off",
  },
};
