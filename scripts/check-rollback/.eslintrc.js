module.exports = {
  env: {
    browser: false,
    es2021: true,
  },
  extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended'],
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 12,
    project: ['./tsconfig.json']
  },
  plugins: ['@typescript-eslint'],
  rules: {
    // enable additional rules
    indent: ['error', 2, {"SwitchCase": 1}],
    'linebreak-style': ['error', 'unix'],
    quotes: ['error', 'single'],
    semi: ['error', 'always'],
  },
  ignorePatterns: [
    '.eslintrc.js',
    '.vscode/**',
    '.yarn/**',
    '**/build/*',
    '**/coverage/*',
    '**/node_modules/*'
  ],
};

