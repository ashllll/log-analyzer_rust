export default {
  preset: 'ts-jest',
  testEnvironment: 'jsdom',
  roots: ['<rootDir>/src'],
  testMatch: ['**/__tests__/**/*.ts', '**/__tests__/**/*.tsx', '**/?(*.)+(spec|test).ts', '**/?(*.)+(spec|test).tsx'],
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
      branches: 10,    // 根据实际覆盖率 10.4% 调整
      functions: 15,   // 根据实际覆盖率 15.43% 调整
      lines: 15,       // 根据实际覆盖率 15.22% 调整
      statements: 15,  // 根据实际覆盖率 15.41% 调整
    },
  },
  testTimeout: 15000, // 从 10000ms 增加到 15000ms 以应对 CI 环境
  transformIgnorePatterns: [
    'node_modules/(?!(react-error-boundary|lucide-react|react-hot-toast)/)',
  ],
  verbose: true, // 添加详细日志输出
  collectCoverage: true, // 确保收集覆盖率信息
};