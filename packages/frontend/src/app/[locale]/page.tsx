'use client';

import { useState } from 'react';
import { useTranslations } from 'next-intl';
import { SearchBar } from '@/components/search/SearchBar';
import { FilterPanel, FilterOptions } from '@/components/search/FilterPanel';
import { ResultList } from '@/components/search/ResultList';
import { searchLaws } from '@/lib/api';
import type { SearchResult, ApiError } from '@/types';

export default function HomePage() {
  const t = useTranslations('home');
  const tSearch = useTranslations('search');

  const [results, setResults] = useState<SearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);

  const [filters, setFilters] = useState<FilterOptions>({
    jurisdictions: ['DE', 'EU'],
    docTypes: [],
  });

  const handleSearch = async (query: string) => {
    setIsLoading(true);
    setError(null);
    setHasSearched(true);

    try {
      const response = await searchLaws({
        q: query,
        jurisdiction: filters.jurisdictions.length === 1 ? filters.jurisdictions[0] : undefined,
        doc_type: filters.docTypes.length === 1 ? filters.docTypes[0] : undefined,
        limit: 20,
      });
      setResults(response.results);
    } catch (err) {
      const apiError = err as ApiError;
      setError(apiError.detail || apiError.details || apiError.error || tSearch('unknownError'));
      setResults([]);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="space-y-8">
      {/* Hero Section */}
      <div className="text-center space-y-4">
        <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
          {t('title')}
        </h1>
        <p className="text-xl text-muted-foreground max-w-2xl mx-auto">
          {t('subtitle')}
        </p>
      </div>

      {/* Search Bar */}
      <div className="max-w-3xl mx-auto">
        <SearchBar onSearch={handleSearch} isLoading={isLoading} />
      </div>

      {/* Main Content */}
      {hasSearched && (
        <div className="grid gap-8 lg:grid-cols-[280px_1fr]">
          {/* Filter Sidebar */}
          <aside className="lg:sticky lg:top-24 lg:self-start">
            <FilterPanel filters={filters} onFiltersChange={setFilters} />
          </aside>

          {/* Results */}
          <div>
            <ResultList
              results={results}
              isLoading={isLoading}
              error={error}
            />
          </div>
        </div>
      )}

      {/* Welcome Message */}
      {!hasSearched && (
        <div className="max-w-4xl mx-auto space-y-6 pt-8">
          <div className="grid gap-4 md:grid-cols-3 lg:grid-cols-5">
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">{t('federalLaw')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('federalLawDesc')}
              </p>
            </div>
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">{t('euLaw')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('euLawDesc')}
              </p>
            </div>
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">{t('frenchLaw')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('frenchLawDesc')}
              </p>
            </div>
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">{t('italianLaw')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('italianLawDesc')}
              </p>
            </div>
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">{t('spanishLaw')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('spanishLawDesc')}
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
