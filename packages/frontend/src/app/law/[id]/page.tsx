import { notFound } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import Link from 'next/link';
import { Button } from '@/components/ui/button';
import { LawViewer } from '@/components/law/LawViewer';
import { apiClient } from '@/lib/api';

interface LawPageProps {
  params: {
    id: string;
  };
}

async function getLaw(id: string) {
  try {
    return await apiClient.getLaw(id);
  } catch (error) {
    return null;
  }
}

export default async function LawPage({ params }: LawPageProps) {
  const law = await getLaw(params.id);

  if (!law) {
    notFound();
  }

  return (
    <div className="space-y-6">
      <div>
        <Link href="/">
          <Button variant="ghost" size="sm" className="gap-2">
            <ArrowLeft className="h-4 w-4" />
            Zurück zur Suche
          </Button>
        </Link>
      </div>

      <LawViewer law={law} />
    </div>
  );
}
