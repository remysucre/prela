
WITH UserBadgeStats AS (
    SELECT 
        U.Id AS UserId,
        COUNT(B.Id) AS TotalBadges,
        SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
    FROM Users U
    LEFT JOIN Badges B ON U.Id = B.UserId
    GROUP BY U.Id
),
PostActivityStats AS (
    SELECT 
        P.OwnerUserId,
        COUNT(P.Id) AS TotalPosts,
        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
        MAX(P.CreationDate) AS MostRecentPost
    FROM Posts P
    GROUP BY P.OwnerUserId
),
ClosedPostReasons AS (
    SELECT 
        PH.UserId,
        PH.Comment AS CloseReason,
        COUNT(PH.Id) AS CloseReasonCount
    FROM PostHistory PH
    WHERE PH.PostHistoryTypeId = 10 
    GROUP BY PH.UserId, PH.Comment
),
FinalStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COALESCE(UB.TotalPosts, 0) AS TotalPosts,
        COALESCE(UB.TotalQuestions, 0) AS TotalQuestions,
        COALESCE(UPB.TotalBadges, 0) AS TotalBadges,
        COALESCE(UPB.GoldBadges, 0) AS GoldBadges,
        COALESCE(UPB.SilverBadges, 0) AS SilverBadges,
        COALESCE(UPB.BronzeBadges, 0) AS BronzeBadges,
        COALESCE(CPR.CloseReasonCount, 0) AS CloseReasonCount
    FROM Users U
    LEFT JOIN PostActivityStats UB ON U.Id = UB.OwnerUserId
    LEFT JOIN UserBadgeStats UPB ON U.Id = UPB.UserId
    LEFT JOIN ClosedPostReasons CPR ON U.Id = CPR.UserId
)
SELECT 
    F.UserId,
    F.DisplayName,
    F.TotalPosts,
    F.TotalQuestions,
    F.TotalBadges,
    F.GoldBadges,
    F.SilverBadges,
    F.BronzeBadges,
    F.CloseReasonCount,
    CASE 
        WHEN F.TotalPosts > 100 THEN 'Elite User'
        WHEN F.TotalPosts BETWEEN 50 AND 100 THEN 'Active User'
        ELSE 'New User'
    END AS UserCategory,
    CASE 
        WHEN F.CloseReasonCount > 5 THEN 'Frequent Closures'
        ELSE 'Rarely Closed'
    END AS ClosureFrequency
FROM FinalStats F
WHERE F.TotalPosts IS NOT NULL
ORDER BY F.TotalPosts DESC, F.UserId, F.CloseReasonCount
LIMIT 100;
