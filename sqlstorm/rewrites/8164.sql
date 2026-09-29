WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.OwnerUserId,
        u.DisplayName AS OwnerDisplayName,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.PostTypeId = 1 AND p.Score > 10
),
TopUsers AS (
    SELECT 
        OwnerUserId,
        COUNT(*) AS PostCount,
        SUM(Score) AS TotalScore
    FROM 
        RankedPosts
    WHERE 
        UserPostRank <= 5
    GROUP BY 
        OwnerUserId
    HAVING 
        COUNT(*) >= 5
),
MostActiveUser AS (
    SELECT 
        u.Id,
        u.DisplayName,
        tu.PostCount,
        tu.TotalScore
    FROM 
        Users u
    JOIN 
        TopUsers tu ON u.Id = tu.OwnerUserId
    ORDER BY 
        tu.TotalScore DESC
    LIMIT 1
)
SELECT 
    mu.DisplayName AS MostActiveUser,
    mu.PostCount,
    mu.TotalScore,
    STRING_AGG(rp.Title, '; ' ORDER BY rp.PostId) AS TopPostTitles
FROM 
    MostActiveUser mu
JOIN 
    RankedPosts rp ON mu.Id = rp.OwnerUserId
GROUP BY 
    mu.DisplayName, mu.PostCount, mu.TotalScore;
