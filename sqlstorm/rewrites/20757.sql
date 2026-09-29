WITH UserBadges AS (
    SELECT 
        U.Id AS UserId,
        COUNT(B.Id) AS TotalBadges,
        STRING_AGG(B.Name, ', ') AS BadgeNames
    FROM Users U
    LEFT JOIN Badges B ON U.Id = B.UserId
    WHERE U.Reputation > 1000
    GROUP BY U.Id
),
PostStatistics AS (
    SELECT 
        P.OwnerUserId,
        COUNT(DISTINCT P.Id) AS TotalPosts,
        SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswers,
        AVG(P.ViewCount) AS AvgViews,
        AVG(P.Score) AS AvgScore
    FROM Posts P
    LEFT JOIN Users U ON P.OwnerUserId = U.Id
    WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
    GROUP BY P.OwnerUserId
),
ClosedPosts AS (
    SELECT 
        PH.PostId,
        PH.CreationDate,
        STRING_AGG(CASE WHEN C.PostId IS NOT NULL THEN 'Closed' ELSE 'Active' END, ' | ') AS PostStatus
    FROM PostHistory PH
    LEFT JOIN Comments C ON PH.PostId = C.PostId 
    WHERE PH.PostHistoryTypeId IN (10, 11)
    GROUP BY PH.PostId, PH.CreationDate
),
UserPostBadgeStats AS (
    SELECT 
        U.Id AS UserId,
        UB.TotalBadges,
        PS.TotalPosts,
        PS.TotalAnswers,
        PS.AvgViews,
        PS.AvgScore,
        CP.PostStatus
    FROM Users U
    LEFT JOIN UserBadges UB ON U.Id = UB.UserId
    LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
    LEFT JOIN ClosedPosts CP ON PS.OwnerUserId = CP.PostId
    WHERE U.Location IS NOT NULL OR U.AboutMe IS NOT NULL
)

SELECT 
    U.DisplayName,
    COALESCE(UB.TotalBadges, 0) AS BadgeCount,
    COALESCE(PS.TotalPosts, 0) AS PostsCount,
    COALESCE(PS.TotalAnswers, 0) AS AnswersCount,
    COALESCE(PS.AvgViews, 0) AS AverageViews,
    COALESCE(PS.AvgScore, 0) AS AverageScore,
    CASE 
        WHEN CP.PostStatus LIKE '%Closed%' THEN 'Discontinued Activity'
        ELSE 'Active Participation'
    END AS UserActivityStatus
FROM Users U
LEFT JOIN UserBadges UB ON U.Id = UB.UserId
LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
LEFT JOIN ClosedPosts CP ON PS.OwnerUserId = CP.PostId
WHERE U.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years'
ORDER BY U.Reputation DESC, U.Id, CP.CreationDate
LIMIT 100;