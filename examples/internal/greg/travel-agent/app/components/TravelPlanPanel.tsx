import { TravelPlanItem, type PlanItem } from "./TravelPlanItem";

interface TravelPlanPanelProps {
  planItems: PlanItem[];
  onExport?: () => void;
}

export function TravelPlanPanel({ planItems, onExport }: TravelPlanPanelProps) {
  return (
    <div className="w-96 flex flex-col bg-white rounded-2xl shadow-xl overflow-hidden">
      {/* Travel Plan Header */}
      <div className="bg-gradient-to-r from-purple-600 to-pink-600 p-4">
        <h2 className="text-xl font-semibold text-white">Travel Plan</h2>
      </div>

      {/* Travel Plan Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4">
        {planItems.length === 0 ? (
          <div className="text-gray-500 text-center py-8">
            <div className="text-6xl mb-4">🗺️</div>
            <p className="text-sm">Your travel itinerary will appear here</p>
          </div>
        ) : (
          <div className="space-y-3">
            {planItems.map((item) => (
              <TravelPlanItem key={item.id} item={item} />
            ))}
          </div>
        )}
      </div>

      {/* Travel Plan Footer */}
      <div className="p-4 bg-gray-50 border-t border-gray-200">
        <button
          onClick={onExport}
          className="w-full px-4 py-3 bg-gradient-to-r from-purple-600 to-pink-600 text-white rounded-xl font-medium hover:shadow-lg transition-all duration-200 hover:scale-105"
        >
          Export Itinerary
        </button>
      </div>
    </div>
  );
}
