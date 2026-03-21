export default {
  preset: 'ts-jest',
  testEnvironment: 'jsdom',
  roots: ['<rootDir>/src'],
  testMatch: ['**/__tests__/**/*.ts', '**/__tests__/**/*.tsx', '**/?(*.)+(spec|test).ts', '**/?(*.)+(spec|test).tsx'],
  testPathIgnorePatterns: ['/node_modules/', '/__mocks__/'],
  transform: {
    '^.+\\.tsx?$': ['ts-jest', {
      tsconfig: {
        jsx: 'react',
        esModuleInterop: true,
      },
    }],
  },
  moduleNameMapper: {
    '\\.(css|less|scss|sass)$': 'identity-obj-proxy',
    '^@/(.*)$': '<rootDir>/src/$1',
  },
  setupFilesAfterEnv: ['<rootDir>/src/setupTests.ts'],
  collectCoverageFrom: [
    'src/**/*.{ts,tsx}',
    '!src/**/*.d.ts',
    '!src/main.tsx',
    '!src/vite-env.d.ts',
  ],
  coverageThreshold: {
    global: {
      branches: 40,    // 阶段性目标：40%，后续逐步提升至 60%+
      functions: 40,
      lines: 40,
      statements: 40,
    },
  },
  testTimeout: 15000, // 从 10000ms 增加到 15000ms 以应对 CI 环境
  transformIgnorePatterns: [
    'node_modules/(?!(react-error-boundary|lucide-react|react-hot-toast)/)',
  ],
  verbose: true, // 添加详细日志输出
  collectCoverage: true, // 确保收集覆盖率信息
};