import * as fs from "fs";
import * as path from "path";
import * as yaml from "js-yaml";
import { loadDocumentTypes } from "./documentDiscovery.js";
import type { DocumentTypeConfig } from "./documentDiscovery.js";

export interface DashboardSection {
    title: string;
    "doc-type": string;
    statuses?: string[];
    categories?: string[];
    error?: string;
}

export interface Dashboard {
    label: string;
    sections: Record<string, DashboardSection>;
    filePath: string;
}

interface RawSection {
    title?: string;
    "doc-type"?: string;
    statuses?: string[];
    categories?: string[];
}

interface RawDashboard {
    label?: string;
    sections?: Record<string, RawSection>;
}

/**
 * Scans `.vector/dashboards/` for YAML files and parses them into Dashboard objects.
 */
export function scanDashboards(workspaceRoot: string): Dashboard[] {
    const dashboardDir = path.join(workspaceRoot, ".vector", "dashboards");
    const docTypesYaml = loadDocumentTypes(workspaceRoot);
    const docTypes = docTypesYaml?.["document-types"] ?? {};

    if (!fs.existsSync(dashboardDir)) {
        return [];
    }

    const results: Dashboard[] = [];
    try {
        const entries = fs.readdirSync(dashboardDir, { withFileTypes: true });
        for (const entry of entries) {
            if (entry.isFile() && (entry.name.endsWith(".yaml") || entry.name.endsWith(".yml"))) {
                const filePath = path.join(dashboardDir, entry.name);
                const dashboard = loadDashboard(filePath, docTypes);
                if (dashboard) {
                    results.push(dashboard);
                }
            }
        }
    } catch {
        // ignore errors reading directory
    }

    return results;
}

/**
 * Loads and validates a single dashboard YAML file.
 * Returns null if the file is completely invalid (e.g. missing label).
 * Individual section errors are captured within the section objects.
 */
export function loadDashboard(
    filePath: string,
    docTypes: Record<string, DocumentTypeConfig> = {},
): Dashboard | null {
    try {
        const content = fs.readFileSync(filePath, "utf-8");
        const raw = yaml.load(content) as RawDashboard | undefined;

        if (!raw || typeof raw.label !== "string") {
            return null;
        }

        const sections: Record<string, DashboardSection> = {};
        if (raw.sections && typeof raw.sections === "object") {
            for (const [key, sectionRaw] of Object.entries(raw.sections)) {
                const section: DashboardSection = {
                    title: sectionRaw.title ?? key,
                    "doc-type": sectionRaw["doc-type"] ?? "",
                };

                if (!section["doc-type"]) {
                    section.error = "Missing doc-type";
                } else {
                    const config = docTypes[section["doc-type"]];
                    if (!config) {
                        section.error = `Unknown doc-type: ${section["doc-type"]}`;
                    } else {
                        validateSectionFilters(section, sectionRaw, config);
                    }
                }

                sections[key] = section;
            }
        }

        return {
            label: raw.label,
            sections,
            filePath,
        };
    } catch {
        return null;
    }
}

function validateSectionFilters(
    section: DashboardSection,
    raw: RawSection,
    config: DocumentTypeConfig,
): void {
    if (config.layout === "status") {
        if (!Array.isArray(raw.statuses)) {
            section.error = "Section for 'status' layout must provide 'statuses' array";
        } else {
            section.statuses = raw.statuses;
            if (raw.categories) {
                section.error = "Section for 'status' layout should not provide 'categories'";
            }
        }
    } else if (config.layout === "category") {
        if (!Array.isArray(raw.categories)) {
            section.error = "Section for 'category' layout must provide 'categories' array";
        } else {
            section.categories = raw.categories;
            if (raw.statuses) {
                section.error = "Section for 'category' layout should not provide 'statuses'";
            }
        }
    } else {
        // layout === "directory"
        if (raw.statuses || raw.categories) {
            section.error = "Section for 'directory' layout should not provide filters";
        }
    }
}
