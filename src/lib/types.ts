export interface Toast {
  id: number;
  type: 'success' | 'error' | 'info';
  message: string;
}

export interface Pentacle {
  id: number;
  url: string;
  title: string | null;
  description: string | null;
  site_url: string | null;
  icon_url: string | null;
  default_quality: number;
  article_count: number;
  unread_count: number;
}

export interface FnordCategoryInfo {
  color: string | null;
  icon: string | null;
  name: string;
}

export interface Fnord {
  id: number;
  pentacle_id: number;
  pentacle_title: string | null;
  guid: string;
  url: string;
  title: string;
  author: string | null;
  content_raw: string | null;
  content_full: string | null;
  summary: string | null;
  image_url: string | null;
  published_at: string | null;
  processed_at: string | null;
  status: "concealed" | "illuminated" | "golden_apple";
  political_bias: number | null;
  sachlichkeit: number | null;
  quality_score: number | null;
  has_changes: boolean;
  changed_at: string | null;
  revision_count: number;
  categories: FnordCategoryInfo[];
}

export interface FnordRevision {
  id: number;
  fnord_id: number;
  title: string;
  author: string | null;
  content_raw: string | null;
  content_full: string | null;
  summary: string | null;
  content_hash: string;
  revision_at: string;
}

export interface FnordFilter {
  pentacle_id?: number;
  sephiroth_id?: number;
  main_sephiroth_id?: number;  // Filter by main category (includes all subcategories)
  status?: string;
  limit?: number;
  offset?: number;
}

export interface FnordStats {
  total_revisions: number;
  articles_with_changes: number;
  by_category: CategoryRevisionStats[];
  by_source: SourceRevisionStats[];
}

export interface CategoryRevisionStats {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  color: string | null;
  revision_count: number;
  article_count: number;
}

export interface SourceRevisionStats {
  pentacle_id: number;
  title: string | null;
  revision_count: number;
  article_count: number;
}

export interface SyncResponse {
  success: boolean;
  results: SyncResultResponse[];
  total_new: number;
  total_updated: number;
}

export interface SyncResultResponse {
  pentacle_id: number;
  pentacle_title: string | null;
  new_articles: number;
  updated_articles: number;
  error: string | null;
}

export interface RetrievalResponse {
  fnord_id: number;
  success: boolean;
  content: string | null;
  error: string | null;
}

export interface OllamaStatus {
  available: boolean;
  models: string[];
  recommended_main: string;
  recommended_embedding: string;
  has_recommended_main: boolean;
  has_recommended_embedding: boolean;
}

export interface SummaryResponse {
  fnord_id: number;
  success: boolean;
  summary: string | null;
  error: string | null;
}

export interface BiasAnalysis {
  political_bias: number;
  sachlichkeit: number;
}

export interface AnalysisResponse {
  fnord_id: number;
  success: boolean;
  analysis: BiasAnalysis | null;
  error: string | null;
}

export interface UnprocessedCount {
  total: number;
  with_content: number;
}

export interface BatchProgress {
  current: number;
  total: number;
  fnord_id: number;
  title: string;
  success: boolean;
  error: string | null;
}

export interface BatchResult {
  processed: number;
  succeeded: number;
  failed: number;
}

export interface EmbeddingProgress {
  queue_size: number;
  total: number;
  processed: number;
  failed: number;
  is_processing: boolean;
}

export interface EmbeddingQueueStatus {
  queue_size: number;
  worker_running: boolean;
  worker_processing: boolean;
}

export interface Sephiroth {
  id: number;
  name: string;
  parent_id: number | null;
  level: number;
  description: string | null;
  color: string | null;
  icon: string | null;
  article_count: number;
}

// Main category (level 0) with aggregated stats
export interface MainCategory {
  id: number;
  name: string;
  icon: string | null;
  color: string | null;
  article_count: number;
  read_count: number;
  percentage: number;
  subcategories: SubCategory[];
}

// Subcategory (level 1) with individual stats
export interface SubCategory {
  id: number;
  name: string;
  icon: string | null;
  parent_id: number;
  article_count: number;
  read_count: number;
  percentage: number;
}

export interface ArticleCategory {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  color: string | null;
  confidence: number;
  source: 'ai' | 'manual';
  assigned_at: string | null;
  parent_id: number | null;
  main_category_name: string | null;
  main_category_color: string | null;
}

export interface Tag {
  id: number;
  name: string;
  count: number;
  last_used: string | null;
}

export interface DiscordianAnalysis {
  summary: string;
  categories: string[];
  keywords: string[];
  political_bias: number;
  sachlichkeit: number;
}

export interface DiscordianResponse {
  fnord_id: number;
  success: boolean;
  analysis: DiscordianAnalysis | null;
  categories_saved: string[];
  tags_saved: string[];
  error: string | null;
}

export interface Keyword {
  id: number;
  name: string;
  count: number;
  article_count: number;
  cluster_id: number | null;
  is_canonical: boolean;
  canonical_id: number | null;
  first_seen: string | null;
  last_used: string | null;
}

export interface KeywordNeighbor {
  id: number;
  name: string;
  cooccurrence: number;
  embedding_similarity: number | null;
  combined_weight: number;
}

export interface KeywordCategory {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  color: string | null;
  weight: number;
  article_count: number;
  parent_id: number | null;
  parent_name: string | null;
}

export interface TrendingKeyword {
  id: number;
  name: string;
  total_count: number;
  recent_count: number;
  growth_rate: number;
}

export interface NetworkStats {
  total_keywords: number;
  total_connections: number;
  total_clusters: number;
  avg_neighbors_per_keyword: number;
}

export interface GraphNode {
  id: number;
  name: string;
  count: number;
  article_count: number;
  cluster_id: number | null;
}

export interface GraphEdge {
  source: number;
  target: number;
  weight: number;
  cooccurrence: number;
}

export interface NetworkGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export type MainView = 'articles' | 'network' | 'fnord' | 'mindfuck';

// Operation Mindfuck (Bias Mirror)
export interface SubCategoryReadStats {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  read_count: number;
  total_count: number;
  percentage: number;
}

export interface CategoryReadStats {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  color: string | null;
  read_count: number;
  total_count: number;
  percentage: number;
  subcategories: SubCategoryReadStats[];
}

export interface BiasReadStats {
  bias_value: number;
  label: string;
  read_count: number;
  percentage: number;
}

export interface SachlichkeitReadStats {
  sachlichkeit_value: number;
  label: string;
  read_count: number;
  percentage: number;
}

export interface ReadingProfile {
  total_read: number;
  total_articles: number;
  read_percentage: number;
  avg_political_bias: number | null;
  avg_sachlichkeit: number | null;
  by_category: CategoryReadStats[];
  by_bias: BiasReadStats[];
  by_sachlichkeit: SachlichkeitReadStats[];
  first_read_at: string | null;
  last_read_at: string | null;
}

export interface BlindSpot {
  spot_type: string;
  name: string;
  description: string;
  severity: string;
  available_count: number;
  read_count: number;
  main_category: string | null;
  main_category_color: string | null;
}

export interface CounterPerspective {
  fnord_id: number;
  title: string;
  pentacle_title: string | null;
  published_at: string | null;
  political_bias: number | null;
  reason: string;
}

export interface ReadingTrend {
  date: string;
  read_count: number;
  avg_bias: number | null;
  avg_sachlichkeit: number | null;
}

// OPML Import/Export
export interface OpmlFeedPreview {
  url: string;
  title: string | null;
  category: string | null;
  already_exists: boolean;
}

export interface OpmlImportResult {
  total_feeds: number;
  imported: number;
  skipped: number;
  errors: string[];
}

// Similar Articles (Phase 3)
export interface SimilarArticleTag {
  id: number;
  name: string;
}

export interface SimilarArticleCategory {
  id: number;
  name: string;
  icon: string | null;
  color: string | null;
}

export interface SimilarArticle {
  fnord_id: number;
  title: string;
  pentacle_title: string | null;
  published_at: string | null;
  similarity: number;
  tags: SimilarArticleTag[];
  categories: SimilarArticleCategory[];
}

export interface SimilarArticlesResponse {
  fnord_id: number;
  similar: SimilarArticle[];
}

export interface ArticleEmbeddingStats {
  total_articles: number;
  with_embedding: number;
  without_embedding: number;
  processable: number;
}

// Semantic Search (Phase 3)
export interface SearchResult {
  fnord_id: number;
  title: string;
  pentacle_title: string | null;
  published_at: string | null;
  summary: string | null;
  similarity: number;
}

export interface SemanticSearchResponse {
  query: string;
  results: SearchResult[];
}
