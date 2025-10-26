import type { TravelAgentContext } from "../store/atoms";

interface ContextPanelProps {
  context: TravelAgentContext;
}

export function ContextPanel({ context }: ContextPanelProps) {
  const hasAnyData =
    context.nAdults !== null ||
    context.nChildren !== null ||
    context.interests.length > 0 ||
    context.homeLocation !== null ||
    context.dateRange !== null;

  return (
    <div className="flex flex-col bg-white rounded-2xl shadow-xl overflow-hidden h-full">
      {/* Context Header */}
      <div className="bg-gradient-to-r from-blue-600 to-cyan-600 p-4">
        <h2 className="text-xl font-semibold text-white">Travel Context</h2>
      </div>

      {/* Context Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {!hasAnyData ? (
          <div className="text-gray-500 text-center py-8">
            <div className="text-6xl mb-4">🧳</div>
            <p className="text-sm">
              Your travel preferences will appear here
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Travelers */}
            {(context.nAdults !== null || context.nChildren !== null) && (
              <div className="bg-gradient-to-br from-blue-50 to-cyan-50 rounded-xl p-4">
                <h3 className="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
                  <span className="text-xl">👥</span>
                  Travelers
                </h3>
                <div className="space-y-2 text-sm">
                  {context.nAdults !== null && (
                    <div className="flex justify-between">
                      <span className="text-gray-600">Adults:</span>
                      <span className="font-medium text-gray-900">
                        {context.nAdults}
                      </span>
                    </div>
                  )}
                  {context.nChildren !== null && (
                    <div className="flex justify-between">
                      <span className="text-gray-600">Children:</span>
                      <span className="font-medium text-gray-900">
                        {context.nChildren}
                      </span>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Home Location */}
            {context.homeLocation && (
              <div className="bg-gradient-to-br from-purple-50 to-pink-50 rounded-xl p-4">
                <h3 className="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
                  <span className="text-xl">🏠</span>
                  Home Location
                </h3>
                <p className="text-sm text-gray-900 font-medium">
                  {context.homeLocation}
                </p>
              </div>
            )}

            {/* Date Range */}
            {context.dateRange && (
              <div className="bg-gradient-to-br from-green-50 to-teal-50 rounded-xl p-4">
                <h3 className="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
                  <span className="text-xl">📅</span>
                  Travel Dates
                </h3>
                <p className="text-sm text-gray-900 font-medium">
                  {context.dateRange}
                </p>
              </div>
            )}

            {/* Interests */}
            {context.interests.length > 0 && (
              <div className="bg-gradient-to-br from-orange-50 to-yellow-50 rounded-xl p-4">
                <h3 className="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
                  <span className="text-xl">🎯</span>
                  Interests
                </h3>
                <div className="flex flex-wrap gap-2">
                  {context.interests.map((interest, index) => (
                    <span
                      key={index}
                      className="px-3 py-1 bg-white rounded-full text-xs font-medium text-gray-700 shadow-sm"
                    >
                      {interest}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
