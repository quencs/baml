export interface PlanItem {
  id: string;
  icon: string;
  title: string;
  description: string;
  date?: string;
}

interface TravelPlanItemProps {
  item: PlanItem;
}

export function TravelPlanItem({ item }: TravelPlanItemProps) {
  return (
    <div className="border border-gray-200 rounded-xl p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start gap-3">
        <div className="text-2xl">{item.icon}</div>
        <div className="flex-1">
          <h3 className="font-semibold text-gray-800">{item.title}</h3>
          <p className="text-sm text-gray-600 mt-1">{item.description}</p>
          {item.date && (
            <p className="text-xs text-gray-500 mt-1">{item.date}</p>
          )}
        </div>
      </div>
    </div>
  );
}
