'use client';

import { useState } from 'react';
import { SearchBar } from '@/components/search/SearchBar';
import { FilterPanel, FilterOptions } from '@/components/search/FilterPanel';
import { ResultList } from '@/components/search/ResultList';
import { apiClient } from '@/lib/api';
import type { SearchResult, ApiError } from '@/types';

export default function HomePage() {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);
  
  const [filters, setFilters] = useState<FilterOptions>({
    jurisdictions: ['DE', 'BY', 'EU'],
    includeCases: false,
  });

  const handleSearch = async (query: string) => {
    setIsLoading(true);
    setError(null);
    setHasSearched(true);

    try {
      const searchResults = await apiClient.search({
        query,
        jurisdictions: filters.jurisdictions,
        includeCases: filters.includeCases,
      });
      
      setResults(searchResults);
    } catch (err) {
      const apiError = err as ApiError;
      setError(
        apiError.details || apiError.error || 'Ein unbekannter Fehler ist aufgetreten'
      );
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
          Legal MCP
        </h1>
        <p className="text-xl text-muted-foreground max-w-2xl mx-auto">
          Semantische Suche für deutsches und EU-Recht
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
        <div className="max-w-3xl mx-auto space-y-6 pt-8">
          <div className="grid gap-4 md:grid-cols-3">
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">Bundesrecht</h3>
              <p className="text-sm text-muted-foreground">
                Durchsuchen Sie deutsche Bundesgesetze und Verordnungen
              </p>
            </div>
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">Landesrecht Bayern</h3>
              <p className="text-sm text-muted-foreground">
                Zugriff auf bayerische Landesgesetze und -verordnungen
              </p>
            </div>
            <div className="text-center p-6 border rounded-lg">
              <h3 className="font-semibold mb-2">EU-Recht</h3>
              <p className="text-sm text-muted-foreground">
                EU-Verordnungen und Richtlinien durchsuchen
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
