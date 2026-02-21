'use client';

import { useTranslations } from 'next-intl';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import type { DocType } from '@/types';

export interface FilterOptions {
  jurisdictions: string[];
  docTypes: DocType[];
}

interface FilterPanelProps {
  filters: FilterOptions;
  onFiltersChange: (filters: FilterOptions) => void;
}

const JURISDICTIONS = ['DE', 'EU', 'FR', 'IT', 'ES'] as const;
const DOC_TYPES: DocType[] = ['statute', 'regulation', 'case', 'directive'];

export function FilterPanel({ filters, onFiltersChange }: FilterPanelProps) {
  const t = useTranslations('filter');

  const handleJurisdictionChange = (jurisdiction: string, checked: boolean) => {
    const newJurisdictions = checked
      ? [...filters.jurisdictions, jurisdiction]
      : filters.jurisdictions.filter((j) => j !== jurisdiction);
    onFiltersChange({ ...filters, jurisdictions: newJurisdictions });
  };

  const handleDocTypeChange = (docType: DocType, checked: boolean) => {
    const newDocTypes = checked
      ? [...filters.docTypes, docType]
      : filters.docTypes.filter((d) => d !== docType);
    onFiltersChange({ ...filters, docTypes: newDocTypes });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{t('title')}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div>
          <h3 className="mb-3 text-sm font-semibold">{t('jurisdictions')}</h3>
          <div className="space-y-2">
            {JURISDICTIONS.map((jurisdiction) => (
              <div key={jurisdiction} className="flex items-center space-x-2">
                <Checkbox
                  id={`jurisdiction-${jurisdiction.toLowerCase()}`}
                  checked={filters.jurisdictions.includes(jurisdiction)}
                  onCheckedChange={(checked) =>
                    handleJurisdictionChange(jurisdiction, checked as boolean)
                  }
                />
                <Label
                  htmlFor={`jurisdiction-${jurisdiction.toLowerCase()}`}
                  className="cursor-pointer"
                >
                  {t(jurisdiction.toLowerCase() as 'de' | 'eu' | 'fr' | 'it' | 'es')}
                </Label>
              </div>
            ))}
          </div>
        </div>

        <Separator />

        <div>
          <h3 className="mb-3 text-sm font-semibold">{t('docTypes')}</h3>
          <div className="space-y-2">
            {DOC_TYPES.map((docType) => (
              <div key={docType} className="flex items-center space-x-2">
                <Checkbox
                  id={`doctype-${docType}`}
                  checked={filters.docTypes.includes(docType)}
                  onCheckedChange={(checked) =>
                    handleDocTypeChange(docType, checked as boolean)
                  }
                />
                <Label htmlFor={`doctype-${docType}`} className="cursor-pointer">
                  {t(docType)}
                </Label>
              </div>
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
