import { withBaml } from "@boundaryml/baml-nextjs-plugin";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */
  logging: {
    fetches: {
      fullUrl: false,
    },
  },
};

export default withBaml()(nextConfig);
