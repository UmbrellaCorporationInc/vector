import {
    scanGovernedDocuments,
    loadDocumentTypes,
    type DocumentTypeConfig,
} from "./documentDiscovery.js";
import type { DashboardSection } from "./dashboardDiscovery.js";

/**
 * A resolved row in a dashboard section table.
 * Strictly follows the RFC 00017 contract: exactly status and slug.
 */
export interface DashboardRow {
    status: string;
    code: string;
    slug: string;
    stem: string; // Used for stable navigation resolving the path dynamically
}

/**
 * Resolves a dashboard section into a list of document rows.
 * Returns an empty array if the section has an error or if no documents match.
 */
export function resolveDashboardSection(
    workspaceRoot: string,
    section: DashboardSection,
    docTypes: Record<string, DocumentTypeConfig>,
): DashboardRow[] {
    if (section.error) {
        return [];
    }

    const config = docTypes[section["doc-type"]];
    if (!config) {
        return [];
    }

    const allDocs = scanGovernedDocuments(workspaceRoot, section["doc-type"], config);

    if (config.layout === "status") {
        const statuses = section.statuses ?? [];
        return allDocs
            .filter((doc) => doc.status !== undefined && statuses.includes(doc.status))
            .map((doc) => ({
                status: doc.status ?? "",
                code: parseInt(doc.code, 10).toString(),
                slug: doc.slug,
                stem: `${section["doc-type"]}-${doc.code}-${doc.slug}`,
            }));
    }

    if (config.layout === "category") {
        const categories = section.categories ?? [];
        return allDocs
            .filter((doc) => doc.category !== undefined && categories.includes(doc.category))
            .map((doc) => ({
                status: doc.category ?? "", // RFC 00017: category layout uses category as status column
                code: parseInt(doc.code, 10).toString(),
                slug: doc.slug,
                stem: `${section["doc-type"]}-${doc.code}-${doc.slug}`,
            }));
    }

    // layout === "directory"
    return allDocs.map((doc) => ({
        status: "", // RFC 00017: directory layout has empty status column
        code: parseInt(doc.code, 10).toString(),
        slug: doc.slug,
        stem: `${section["doc-type"]}-${doc.code}-${doc.slug}`,
    }));
}

/**
 * Helper to resolve all sections of a dashboard.
 * Used by the viewer to aggregate data.
 */
export function resolveDashboard(
    workspaceRoot: string,
    dashboardSections: Record<string, DashboardSection>,
): Record<string, DashboardRow[]> {
    const docTypesYaml = loadDocumentTypes(workspaceRoot);
    const docTypes = docTypesYaml?.["document-types"] ?? {};
    const results: Record<string, DashboardRow[]> = {};

    for (const [key, section] of Object.entries(dashboardSections)) {
        results[key] = resolveDashboardSection(workspaceRoot, section, docTypes);
    }

    return results;
}
