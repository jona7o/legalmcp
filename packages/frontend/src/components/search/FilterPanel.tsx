'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';

export interface FilterOptions {
  jurisdictions: string[];
  includeCases: boolean;
}

interface FilterPanelProps {
  filters: FilterOptions;
  onFiltersChange: (filters: FilterOptions) => void;
}

export function FilterPanel({ filters, onFiltersChange }: FilterPanelProps) {
  const handleJurisdictionChange = (jurisdiction: string, checked: boolean) => {
    const newJurisdictions = checked
      ? [...filters.jurisdictions, jurisdiction]
      : filters.jurisdictions.filter((j) => j !== jurisdiction);

    onFiltersChange({
      ...filters,
      jurisdictions: newJurisdictions,
    });
  };

  const handleIncludeCasesChange = (checked: boolean) => {
    onFiltersChange({
      ...filters,
      includeCases: checked,
    });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Filter</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div>
          <h3 className="mb-3 text-sm font-semibold">Rechtsgebiete</h3>
          <div className="space-y-2">
            <div className="flex items-center space-x-2">
              <Checkbox
                id="jurisdiction-de"
                checked={filters.jurisdictions.includes('DE')}
                onCheckedChange={(checked) =>
                  handleJurisdictionChange('DE', checked as boolean)
                }
              />
              <Label htmlFor="jurisdiction-de" className="cursor-pointer">
                Deutschland (Bundesrecht)
              </Label>
            </div>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="jurisdiction-by"
                checked={filters.jurisdictions.includes('BY')}
                onCheckedChange={(checked) =>
                  handleJurisdictionChange('BY', checked as boolean)
                }
              />
              <Label htmlFor="jurisdiction-by" className="cursor-pointer">
                Bayern (Landesrecht)
              </Label>
            </div>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="jurisdiction-eu"
                checked={filters.jurisdictions.includes('EU')}
                onCheckedChange={(checked) =>
                  handleJurisdictionChange('EU', checked as boolean)
                }
              />
              <Label htmlFor="jurisdiction-eu" className="cursor-pointer">
                Europäische Union
              </Label>
            </div>
          </div>
        </div>

        <Separator />

        <div>
          <h3 className="mb-3 text-sm font-semibold">Dokumenttypen</h3>
          <div className="space-y-2">
            <div className="flex items-center space-x-2">
              <Checkbox
                id="include-cases"
                checked={filters.includeCases}
                onCheckedChange={(checked) =>
                  handleIncludeCasesChange(checked as boolean)
                }
              />
              <Label htmlFor="include-cases" className="cursor-pointer">
                Rechtsprechung einschließen
              </Label>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
