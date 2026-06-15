export default {
  preset: "ts-jest",
  testEnvironment: "jsdom",
  roots: ["<rootDir>/src"],
  testMatch: [
    "**/__tests__/**/*.ts",
    "**/__tests__/**/*.tsx",
    "**/?(*.)+(spec|test).ts",
    "**/?(*.)+(spec|test).tsx",
  ],
  testPathIgnorePatterns: ["/node_modules/", "/__mocks__/"],
  transform: {
    "^.+\\.tsx?$": [
      "ts-jest",
      {
        tsconfig: {
          jsx: "react",
          esModuleInterop: true,
        },
      },
    ],
  },
  moduleNameMapper: {
    "\\.(css|less|scss|sass)$": "identity-obj-proxy",
    "^@/(.*)$": "<rootDir>/src/$1",
  },
  setupFilesAfterEnv: ["<rootDir>/src/setupTests.ts"],
  collectCoverageFrom: [
    "src/**/*.{ts,tsx}",
    "!src/**/*.d.ts",
    "!src/main.tsx",
    "!src/vite-env.d.ts",
    "!src/__tests__/e2e/**",
  ],
  coverageThreshold: {
    global: {
      // Ratchet from the measured 2026-06-15 baseline. Raise these values as
      // coverage improves; do not restore an aspirational threshold that has
      // never passed in CI.
      branches: 30,
      functions: 33,
      lines: 38,
      statements: 37,
    },
  },
  testTimeout: 15000, // 从 10000ms 增加到 15000ms 以应对 CI 环境
  transformIgnorePatterns: [
    "node_modules/(?!(react-error-boundary|lucide-react|react-hot-toast)/)",
  ],
  verbose: true, // 添加详细日志输出
  // ME-50: Removed collectCoverage: true to avoid forcing coverage collection on every test run
};
