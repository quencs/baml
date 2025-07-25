import type { NextConfig } from 'next';
import { withBaml } from '@boundaryml/baml-nextjs-plugin';

const nextConfig: NextConfig = {
  /* config options here */
};

export default withBaml()(nextConfig);
