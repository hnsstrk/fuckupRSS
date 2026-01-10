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
  description: string | null;
  color: string | null;
  icon: string | null;
  article_count: number;
}

export interface ArticleCategory {
  sephiroth_id: number;
  name: string;
  icon: string | null;
  color: string | null;
  confidence: number;
  source: 'ai' | 'manual';
  assigned_at: string | null;
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

export type MainView = 'articles' | 'network' | 'fnord';
