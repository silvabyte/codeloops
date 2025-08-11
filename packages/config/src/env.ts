//this is a light wrapper around process.env get/set, etc operations
export const env = {
  get: (key: string) => process.env[key],
  has: (key: string) => process.env[key] !== undefined,
  set: (key: string, value: string) => (process.env[key] = value),
  delete: (key: string) => (process.env[key] = undefined),
};
