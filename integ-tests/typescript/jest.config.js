export default {
  preset: "ts-jest",
  testEnvironment: "node",
  roots: ["<rootDir>/tests"],
  testMatch: ["**/*.test.ts"],
  moduleFileExtensions: ["ts", "js", "json", "node"],
  setupFilesAfterEnv: ["<rootDir>/tests/test-setup.ts"],
  testTimeout: 600000,
  // detectOpenHandles: true,
  // moduleNameMapper: {
  //   "^@/(.*)$": "<rootDir>/$1",
  // },
  // transform: {
  //   '.*\\.(j|t)sx?$': ['@boundaryml/baml'],
  // },
  transform: {}
  // transformIgnorePatterns: ['/node_modules/(?!@boundaryml/baml)'] 
};
