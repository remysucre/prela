
WITH RankedPosts AS (
    SELECT 
        P.Id,
        P.Title,
        P.CreationDate,
        P.Score,
        P.ViewCount,
        U.DisplayName AS OwnerDisplayName,
        RANK() OVER (PARTITION BY EXTRACT(YEAR FROM P.CreationDate) ORDER BY P.Score DESC, P.ViewCount DESC) AS Rank
    FROM 
        Posts P
    JOIN 
        Users U ON P.OwnerUserId = U.Id 
    WHERE 
        P.PostTypeId = 1 
    AND 
        P.CreationDate >= DATE '2024-10-01' - INTERVAL '2 years'
),
PostStatistics AS (
    SELECT 
        RP.OwnerDisplayName,
        COUNT(DISTINCT RP.Id) AS TotalPosts,
        COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS TotalClosed,
        COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS TotalReopened,
        AVG(RP.Score) AS AvgScore,
        AVG(RP.ViewCount) AS AvgViewCount
    FROM 
        RankedPosts RP
    LEFT JOIN 
        PostHistory PH ON RP.Id = PH.PostId
    GROUP BY 
        RP.OwnerDisplayName
)
SELECT 
    PS.OwnerDisplayName,
    PS.TotalPosts,
    PS.TotalClosed,
    PS.TotalReopened,
    PS.AvgScore,
    PS.AvgViewCount
FROM 
    PostStatistics PS
WHERE 
    PS.TotalPosts > 0
ORDER BY 
    PS.AvgScore DESC,
    PS.TotalPosts DESC,
    PS.OwnerDisplayName
LIMIT 10;
