#!/usr/bin/env tsx

import { z } from 'zod';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { parse as parseYaml } from 'yaml';
import matter from 'gray-matter';
import _ from 'lodash';

// Zod schema for the docs.yml navigation structure
const TabSchema = z.object({
  'display-name': z.string().optional(),
  icon: z.string().optional(),
  slug: z.string().optional(),
  href: z.string().optional(),
});

const PageSchema = z.object({
  page: z.string(),
  path: z.string(),
  icon: z.string().optional(),
  slug: z.string().optional(),
});

const SectionSchema: z.ZodType<any> = z.lazy(() => z.object({
  section: z.string(),
  icon: z.string().optional(),
  slug: z.string().optional(),
  contents: z.array(z.union([PageSchema, SectionSchema])),
}));

const NavigationItemSchema = z.object({
  tab: z.string(),
  layout: z.array(z.union([PageSchema, SectionSchema])).optional(),
});

const DocsConfigSchema = z.object({
  title: z.string(),
  tabs: z.record(z.string(), TabSchema),
  navigation: z.array(NavigationItemSchema),
});

// Interface for sitemap entry
interface SitemapEntry {
  title: string;
  path: string;
  section?: string;
  // MDX frontmatter metadata
  slug?: string;
  description?: string;
  layout?: string;
  'hide-toc'?: boolean;
  [key: string]: any; // Allow other frontmatter fields
}

// Function to extract frontmatter from MDX files
function extractMdxMetadata(filePath: string): Record<string, any> {
  try {
    const content = readFileSync(filePath, 'utf-8');
    const { data } = matter(content);
    return data;
  } catch (error) {
    console.warn(`Failed to read MDX file: ${filePath}`, error);
    return {};
  }
}

// Helper function to slugify text (matches Python implementation, preserves underscores)
function slugify(text: string): string {
  // Replace non-alphanumeric characters with spaces (preserving underscores as alnum)
  const normalized = text.split('').map(c => /[a-zA-Z0-9_]/.test(c) ? c : ' ').join('');
  
  // Pattern to match: consecutive caps, title case words, single caps, numbers (including underscores in words)
  const pattern = /[A-Z]{2,}(?=[A-Z][a-z_]+|[0-9]|\s|$)|[A-Z]?[a-z_]+|[A-Z]|[0-9_]+/g;
  const words = normalized.match(pattern) || [];
  
  return words.join('-').toLowerCase();
}

// Function to generate slug from tab/section/title
function generateSlug(tabSlug: string, sectionPath: string | undefined, title: string): string {
  const parts = [tabSlug];
  
  if (sectionPath) {
    // Extract only the section parts (remove tab display name)
    const sectionOnly = sectionPath.split(' > ').slice(1); // Remove first part which is tab display name
    
    // Convert each section to slug format
    sectionOnly.forEach(section => {
      const sectionSlug = slugify(section);
      
      if (sectionSlug) {
        parts.push(sectionSlug);
      }
    });
  }
  
  // Convert title to slug format using slugify helper
  const titleSlug = slugify(title);
  
  if (titleSlug) {
    parts.push(titleSlug);
  }
  
  return '/' + parts.join('/');
}

// Recursive function to process navigation items
function processNavigationItem(
  item: z.infer<typeof PageSchema> | z.infer<typeof SectionSchema>,
  tabDisplayName: string,
  tabSlug: string,
  sectionDisplayPath: string | undefined,
  sectionSlugPath: string | undefined,
  docsRoot: string
): SitemapEntry[] {
  const entries: SitemapEntry[] = [];

  if ('page' in item) {
    // Handle page item
    const mdxPath = join(docsRoot, item.path);
    const metadata = extractMdxMetadata(mdxPath);
    
    const fullSectionPath = sectionDisplayPath ? `${tabDisplayName} > ${sectionDisplayPath}` : undefined;
    const slugSectionPath = sectionSlugPath ? `${tabSlug} > ${sectionSlugPath}` : undefined;
    
    // Determine slug: docs.yml slug > frontmatter slug > generated slug
    let finalSlug = item.slug || metadata.slug;
    if (!finalSlug) {
      finalSlug = generateSlug(tabSlug, slugSectionPath, item.page);
    } else if (!finalSlug.startsWith('/')) {
      // If slug is relative, prefix it with tab and section
      finalSlug = generateSlug(tabSlug, slugSectionPath, finalSlug);
    }
    
    entries.push({
      title: item.page,
      path: item.path,
      section: fullSectionPath,
      ...metadata, // Spread all frontmatter metadata
      slug: finalSlug,
    });
  } else if ('section' in item) {
    // Handle section item - recursively process contents
    const sectionDisplayName = sectionDisplayPath ? `${sectionDisplayPath} > ${item.section}` : item.section;
    const sectionSlugName = sectionSlugPath ? `${sectionSlugPath} > ${item.slug || slugify(item.section)}` : (item.slug || slugify(item.section));
    
    for (const contentItem of item.contents) {
      entries.push(...processNavigationItem(contentItem, tabDisplayName, tabSlug, sectionDisplayName, sectionSlugName, docsRoot));
    }
  }

  return entries;
}

// Main function to generate sitemap
function generateSitemap(docsYmlPath: string): SitemapEntry[] {
  const docsRoot = dirname(docsYmlPath);
  
  // Read and parse docs.yml
  const docsContent = readFileSync(docsYmlPath, 'utf-8');
  const docsConfig = parseYaml(docsContent);
  
  // Validate with Zod schema
  const validatedConfig = DocsConfigSchema.parse(docsConfig);
  
  const sitemap: SitemapEntry[] = [];
  
  // Process each navigation item
  for (const navItem of validatedConfig.navigation) {
    const tabInfo = validatedConfig.tabs[navItem.tab];
    const tabDisplayName = tabInfo?.['display-name'] || navItem.tab;
    const tabSlug = tabInfo?.slug || navItem.tab;
    
    // Skip tabs without layout (like external links)
    if (!navItem.layout) {
      continue;
    }
    
    for (const layoutItem of navItem.layout) {
      const entries = processNavigationItem(layoutItem, tabDisplayName, tabSlug, undefined, undefined, docsRoot);
      sitemap.push(...entries);
    }
  }
  
  return sitemap;
}

// CLI usage
if (require.main === module) {
  const docsYmlPath = process.argv[2] || '/Users/sam/baml2/fern/docs.yml';
  
  try {
    const sitemap = generateSitemap(docsYmlPath);
    console.log(JSON.stringify(sitemap, null, 2));
  } catch (error) {
    console.error('Error generating sitemap:', error);
    process.exit(1);
  }
}

export { generateSitemap, DocsConfigSchema, type SitemapEntry };