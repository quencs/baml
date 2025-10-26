import type { TravelAgentContext } from "../store/atoms";

interface ContextPanelProps {
  context: TravelAgentContext;
}

export function ContextPanel({ context }: ContextPanelProps) {
  const hasAnyData =
    context.nAdults !== null ||
    context.nChildren !== null ||
    (context.interests?.length ?? 0) > 0 ||
    context.homeLocation !== null ||
    context.dateRange !== null;

  return (
    <div className="flex flex-col bg-white rounded-2xl shadow-xl overflow-hidden h-full">
      {/* Context Header */}
      <div className="bg-gradient-to-r from-blue-600 to-cyan-600 p-3">
        <h2 className="text-lg font-semibold text-white">Travel Context</h2>
      </div>

      {/* Context Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {!hasAnyData ? (
          <div className="text-gray-500 text-center py-4">
            <div className="text-4xl mb-2">🧳</div>
            <p className="text-xs">Your travel preferences will appear here</p>
          </div>
        ) : (
          <div className="space-y-2">
            {/* Travelers */}
            {(context.nAdults !== null || context.nChildren !== null) && (
              <div className="bg-gradient-to-br from-blue-50 to-cyan-50 rounded-lg p-3">
                <h3 className="text-xs font-semibold text-gray-700 mb-2 flex items-center gap-1">
                  <span className="text-base">👥</span>
                  Travelers
                </h3>
                <div className="flex gap-4 text-xs">
                  {context.nAdults !== null && (
                    <div className="flex gap-1">
                      <span className="text-gray-600">Adults:</span>
                      <span className="font-medium text-gray-900">
                        {context.nAdults}
                      </span>
                    </div>
                  )}
                  {context.nChildren !== null && (
                    <div className="flex gap-1">
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
              <div className="bg-gradient-to-br from-purple-50 to-pink-50 rounded-lg p-3">
                <h3 className="text-xs font-semibold text-gray-700 mb-1 flex items-center gap-1">
                  <span className="text-base">🏠</span>
                  Home Location
                </h3>
                <p className="text-xs text-gray-900 font-medium">
                  {context.homeLocation}
                </p>
              </div>
            )}

            {/* Date Range */}
            {context.dateRange && (
              <div className="bg-gradient-to-br from-green-50 to-teal-50 rounded-lg p-3">
                <h3 className="text-xs font-semibold text-gray-700 mb-1 flex items-center gap-1">
                  <span className="text-base">📅</span>
                  Travel Dates
                </h3>
                <p className="text-xs text-gray-900 font-medium">
                  {context.dateRange}
                </p>
              </div>
            )}

            {/* Interests */}
            {(context.interests?.length ?? 0) > 0 && (
              <div className="bg-gradient-to-br from-orange-50 to-yellow-50 rounded-lg p-3">
                <h3 className="text-xs font-semibold text-gray-700 mb-2 flex items-center gap-1">
                  <span className="text-base">🎯</span>
                  Interests
                </h3>
                <div className="flex flex-wrap gap-1">
                  {context.interests?.map((interest, index) => (
                    <span
                      key={index}
                      className="px-2 py-0.5 bg-white rounded-full text-xs font-medium text-gray-700 shadow-sm"
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
