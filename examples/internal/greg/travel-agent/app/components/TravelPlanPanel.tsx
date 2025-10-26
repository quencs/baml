import { TravelPlanItem, type PlanItem } from "./TravelPlanItem";
import type { BAMLItinerary } from "../store/atoms";

interface TravelPlanPanelProps {
  planItems: PlanItem[];
  bamlItinerary?: BAMLItinerary;
  onExport?: () => void;
}

export function TravelPlanPanel({ planItems, bamlItinerary, onExport }: TravelPlanPanelProps) {
  const hasBAMLContent = bamlItinerary && (bamlItinerary.flights.length > 0 || bamlItinerary.activities.length > 0);
  return (
    <div className="flex flex-col bg-white rounded-2xl shadow-xl overflow-hidden h-full">
      {/* Travel Plan Header */}
      <div className="bg-gradient-to-r from-purple-600 to-pink-600 p-4">
        <h2 className="text-xl font-semibold text-white">Travel Plan</h2>
      </div>

      {/* Travel Plan Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4">
        {!hasBAMLContent && planItems.length === 0 ? (
          <div className="text-gray-500 text-center py-8">
            <div className="text-6xl mb-4">🗺️</div>
            <p className="text-sm">Your travel itinerary will appear here</p>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Display BAML Itinerary */}
            {hasBAMLContent && (
              <>
                {bamlItinerary.flights.length > 0 && (
                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold text-gray-700 uppercase tracking-wide">Flights</h3>
                    {bamlItinerary.flights.map((flight, index) => (
                      <div key={`flight-${index}`} className="border border-gray-200 rounded-xl p-4 hover:shadow-md transition-shadow">
                        <div className="flex items-start gap-3">
                          <div className="text-2xl">✈️</div>
                          <div className="flex-1">
                            <h3 className="font-semibold text-gray-800">{flight.source} → {flight.dest}</h3>
                            <p className="text-sm text-gray-600 mt-1">{flight.datetime}</p>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {bamlItinerary.activities.length > 0 && (
                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold text-gray-700 uppercase tracking-wide">Activities</h3>
                    {bamlItinerary.activities.map((activity, index) => (
                      <div key={`activity-${index}`} className="border border-gray-200 rounded-xl p-4 hover:shadow-md transition-shadow">
                        <div className="flex items-start gap-3">
                          <div className="text-2xl">🎯</div>
                          <div className="flex-1">
                            <h3 className="font-semibold text-gray-800">{activity.name}</h3>
                            <p className="text-sm text-gray-600 mt-1">{activity.location}</p>
                            <p className="text-xs text-gray-500 mt-1">${activity.price_dollars}</p>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </>
            )}

            {/* Display legacy PlanItems */}
            {planItems.length > 0 && (
              <div className="space-y-3">
                {planItems.map((item) => (
                  <TravelPlanItem key={item.id} item={item} />
                ))}
              </div>
            )}
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
