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
      // 当前实测覆盖率：statements ~31.5%，branches ~27.9%，lines ~31.5%，functions ~30.2%
      // 阈值逐步上调，防止覆盖率倒退
      branches: 25,
      functions: 28,
      lines: 30,
      statements: 30,
    },
  },
  testTimeout: 15000, // 从 10000ms 增加到 15000ms 以应对 CI 环境
  transformIgnorePatterns: [
    'node_modules/(?!(react-error-boundary|lucide-react|react-hot-toast)/)',
  ],
  verbose: true, // 添加详细日志输出
  collectCoverage: true, // 确保收集覆盖率信息
};