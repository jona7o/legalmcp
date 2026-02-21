import { redirect } from 'next/navigation';

// Cases are documents with doc_type='case' stored in the same documents table.
// This page simply redirects to the law detail page using the same document ID.
interface CasePageProps {
  params: Promise<{
    id: string;
    locale: string;
  }>;
}

export default async function CasePage({ params }: CasePageProps) {
  const { id, locale } = await params;
  redirect(`/${locale}/law/${id}`);
}
